use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, header::FORWARDED};
use forwarded_header_value::ForwardedHeaderValue;
use governor::{clock::QuantaInstant, middleware::NoOpMiddleware};
use ipnet::IpNet;
use tawai_core::utils::logger;
use tokio::task::JoinHandle;
use tower_governor::{
    GovernorError, GovernorLayer,
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::KeyExtractor,
};

pub type TawaiGovernorConfig =
    GovernorConfig<TrustedProxyIpKeyExtractor, NoOpMiddleware<QuantaInstant>>;

pub type TawaiGovernorLayer =
    GovernorLayer<TrustedProxyIpKeyExtractor, NoOpMiddleware<QuantaInstant>, axum::body::Body>;

const X_REAL_IP: &str = "x-real-ip";
const X_FORWARDED_FOR: &str = "x-forwarded-for";

#[derive(Debug, Clone)]
pub struct TrustedProxyIpKeyExtractor {
    trusted: Arc<[IpNet]>,
}

impl TrustedProxyIpKeyExtractor {
    pub fn from_env() -> Self {
        let raw = std::env::var("TAWAI_TRUSTED_PROXY").unwrap_or_default();
        Self::from_list(&raw)
    }

    pub fn from_list(raw: &str) -> Self {
        let mut trusted: Vec<IpNet> = vec![
            "127.0.0.1/32".parse().expect("valid IPv4 loopback network"),
            "::1/128".parse().expect("valid IPv6 loopback network"),
        ];
        trusted.extend(
            raw.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|s| match parse_network(s) {
                    Some(net) => Some(net),
                    None => {
                        logger::warn(&format!("Ignoring invalid TAWAI_TRUSTED_PROXY entry: {s}"));
                        None
                    }
                }),
        );
        Self {
            trusted: trusted.into(),
        }
    }
}

impl KeyExtractor for TrustedProxyIpKeyExtractor {
    type Key = IpAddr;

    fn name(&self) -> &'static str {
        "trusted proxy IP"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let peer = peer_ip(req).ok_or(GovernorError::UnableToExtractKey)?;
        if !is_trusted(peer, &self.trusted) {
            return Ok(peer);
        }

        let headers = req.headers();
        if let Some(ip) = select_client_ip(&forwarded_for_chain(headers), &self.trusted) {
            return Ok(ip);
        }
        if let Some(ip) = x_real_ip(headers) {
            return Ok(ip);
        }
        if let Some(ip) = select_client_ip(&forwarded_chain(headers), &self.trusted) {
            return Ok(ip);
        }

        Ok(peer)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

#[derive(Debug)]
pub struct RateLimiters {
    pub global: Arc<TawaiGovernorConfig>,
    pub auth: Arc<TawaiGovernorConfig>,
    pub public: Arc<TawaiGovernorConfig>,
    cleanup: JoinHandle<()>,
}

impl RateLimiters {
    pub fn new() -> Self {
        let extractor = TrustedProxyIpKeyExtractor::from_env();
        let global = Arc::new(global_config(&extractor));
        let auth = Arc::new(auth_config(&extractor));
        let public = Arc::new(public_config(&extractor));
        let cleanup = spawn_cleanup(global.clone(), auth.clone(), public.clone());
        Self {
            global,
            auth,
            public,
            cleanup,
        }
    }

    pub fn global_layer(&self) -> TawaiGovernorLayer {
        GovernorLayer::new(self.global.clone())
    }

    pub fn auth_layer(&self) -> TawaiGovernorLayer {
        GovernorLayer::new(self.auth.clone())
    }

    pub fn public_layer(&self) -> TawaiGovernorLayer {
        GovernorLayer::new(self.public.clone())
    }
}

impl Default for RateLimiters {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RateLimiters {
    fn drop(&mut self) {
        self.cleanup.abort();
    }
}

fn global_config(extractor: &TrustedProxyIpKeyExtractor) -> TawaiGovernorConfig {
    GovernorConfigBuilder::default()
        .per_millisecond(200)
        .burst_size(20)
        .key_extractor(extractor.clone())
        .finish()
        .unwrap()
}

fn auth_config(extractor: &TrustedProxyIpKeyExtractor) -> TawaiGovernorConfig {
    GovernorConfigBuilder::default()
        .per_second(30)
        .burst_size(5)
        .key_extractor(extractor.clone())
        .finish()
        .unwrap()
}

fn public_config(extractor: &TrustedProxyIpKeyExtractor) -> TawaiGovernorConfig {
    GovernorConfigBuilder::default()
        .per_millisecond(500)
        .burst_size(20)
        .key_extractor(extractor.clone())
        .finish()
        .unwrap()
}

fn spawn_cleanup(
    global: Arc<TawaiGovernorConfig>,
    auth: Arc<TawaiGovernorConfig>,
    public: Arc<TawaiGovernorConfig>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            global.limiter().retain_recent();
            auth.limiter().retain_recent();
            public.limiter().retain_recent();
        }
    })
}

fn parse_network(value: &str) -> Option<IpNet> {
    value
        .parse::<IpNet>()
        .ok()
        .or_else(|| value.parse::<IpAddr>().ok().map(IpNet::from))
}

fn peer_ip<T>(req: &Request<T>) -> Option<IpAddr> {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|addr| normalize(addr.ip()))
        .or_else(|| {
            req.extensions()
                .get::<SocketAddr>()
                .map(|addr| normalize(addr.ip()))
        })
}

fn is_trusted(ip: IpAddr, trusted: &[IpNet]) -> bool {
    let ip = normalize(ip);
    trusted.iter().any(|net| net.contains(&ip))
}

fn select_client_ip(chain: &[IpAddr], trusted: &[IpNet]) -> Option<IpAddr> {
    chain
        .iter()
        .rev()
        .copied()
        .find(|ip| !is_trusted(*ip, trusted))
        .or_else(|| chain.first().copied())
        .map(normalize)
}

fn forwarded_for_chain(headers: &HeaderMap) -> Vec<IpAddr> {
    let mut chain = Vec::new();
    for value in headers.get_all(X_FORWARDED_FOR) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        chain.extend(value.split(',').filter_map(parse_ip));
    }
    chain
}

fn x_real_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(X_REAL_IP)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip)
}

fn forwarded_chain(headers: &HeaderMap) -> Vec<IpAddr> {
    let mut chain = Vec::new();
    for value in headers.get_all(FORWARDED) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        let Ok(forwarded) = ForwardedHeaderValue::from_forwarded(value) else {
            continue;
        };
        chain.extend(
            forwarded
                .iter()
                .filter_map(|stanza| stanza.forwarded_for_ip())
                .map(normalize),
        );
    }
    chain
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    let value = value.trim().trim_matches('"');
    value
        .parse::<IpAddr>()
        .ok()
        .or_else(|| value.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
        .or_else(|| {
            value
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse::<IpAddr>().ok())
        })
        .map(normalize)
}

fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn extractor(trusted: &str) -> TrustedProxyIpKeyExtractor {
        TrustedProxyIpKeyExtractor::from_list(trusted)
    }

    fn request(peer: &str, headers: &[(&str, &str)]) -> Request<Body> {
        let mut builder = Request::builder().uri("/");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let mut request = builder.body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        request
    }

    #[test]
    fn untrusted_peer_ignores_forwarded_headers() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "203.0.113.9:1234",
            &[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "5.6.7.8")],
        );

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_uses_rightmost_untrusted_forwarded_for_ip() {
        let extractor = extractor("10.0.0.0/8");
        let request = request(
            "10.0.0.2:1234",
            &[("x-forwarded-for", "1.2.3.4, 6.6.6.6, 10.0.0.1")],
        );

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "6.6.6.6".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_uses_x_real_ip() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[("x-real-ip", "5.6.7.8")]);

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "5.6.7.8".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_uses_forwarded_header() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("forwarded", "for=1.2.3.4, for=6.6.6.6")],
        );

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "6.6.6.6".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusted_peer_falls_back_to_peer_without_headers() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[]);

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "10.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn normalizes_ipv4_mapped_ipv6_peer() {
        let extractor = extractor("127.0.0.1");
        let request = request("[::ffff:127.0.0.1]:1234", &[]);

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "127.0.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn trusts_loopback_by_default() {
        let extractor = extractor("");
        let request = request("127.0.0.1:1234", &[("x-forwarded-for", "6.6.6.6")]);

        assert_eq!(
            extractor.extract(&request).unwrap(),
            "6.6.6.6".parse::<IpAddr>().unwrap()
        );
    }
}
