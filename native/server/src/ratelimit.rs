use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, HeaderName, Request};
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
const FORWARDED_HEADER: &str = "forwarded";
const AUTO_FORWARDING_HEADERS: [&str; 3] = [X_FORWARDED_FOR, X_REAL_IP, FORWARDED_HEADER];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientKey {
    Ip(IpAddr),
    Unknown(IpAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderKind {
    Single,
    Chain,
    Forwarded,
}

#[derive(Debug, Clone)]
struct ForwardingHeader {
    name: HeaderName,
    kind: HeaderKind,
}

impl ForwardingHeader {
    fn new(name: HeaderName) -> Self {
        let kind = match name.as_str() {
            FORWARDED_HEADER => HeaderKind::Forwarded,
            X_REAL_IP => HeaderKind::Single,
            _ => HeaderKind::Chain,
        };
        Self { name, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderLookup {
    Absent,
    Ip(IpAddr),
    Unusable,
}

#[derive(Debug, Clone)]
pub struct TrustedProxyIpKeyExtractor {
    trusted: Arc<[IpNet]>,
    forwarded_headers: Arc<[ForwardingHeader]>,
}

impl TrustedProxyIpKeyExtractor {
    pub fn from_env() -> Self {
        let trusted = std::env::var("TAWAI_TRUSTED_PROXY").unwrap_or_default();
        let forwarded = std::env::var("TAWAI_FORWARDED_HEADER").unwrap_or_default();
        Self::from_config(&trusted, &forwarded)
    }

    pub fn from_list(raw: &str) -> Self {
        Self::from_list_with_headers(raw, default_forwarding_headers())
    }

    fn from_config(trusted_raw: &str, forwarded_raw: &str) -> Self {
        Self::from_list_with_headers(trusted_raw, parse_forwarding_headers(forwarded_raw))
    }

    fn from_list_with_headers(trusted_raw: &str, forwarded_headers: Vec<ForwardingHeader>) -> Self {
        let mut trusted: Vec<IpNet> = vec![
            "127.0.0.1/32".parse().expect("valid IPv4 loopback network"),
            "::1/128".parse().expect("valid IPv6 loopback network"),
        ];
        trusted.extend(
            trusted_raw
                .split(',')
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
            trusted: trusted.into_iter().map(normalize_cidr).collect(),
            forwarded_headers: forwarded_headers.into(),
        }
    }
}

impl KeyExtractor for TrustedProxyIpKeyExtractor {
    type Key = ClientKey;

    fn name(&self) -> &'static str {
        "trusted proxy IP"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let peer = peer_ip(req).ok_or(GovernorError::UnableToExtractKey)?;
        if !is_trusted(peer, &self.trusted) {
            return Ok(ClientKey::Ip(peer));
        }

        for header in self.forwarded_headers.iter() {
            match lookup_client_ip(req.headers(), header, &self.trusted) {
                HeaderLookup::Absent => continue,
                HeaderLookup::Ip(ip) => return Ok(ClientKey::Ip(ip)),
                HeaderLookup::Unusable => return Ok(ClientKey::Unknown(peer)),
            }
        }

        Ok(ClientKey::Ip(peer))
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(match key {
            ClientKey::Ip(ip) => ip.to_string(),
            ClientKey::Unknown(peer) => format!("unknown:{peer}"),
        })
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
        .map(|addr| normalize_ip(addr.ip()))
        .or_else(|| {
            req.extensions()
                .get::<SocketAddr>()
                .map(|addr| normalize_ip(addr.ip()))
        })
}

fn is_trusted(ip: IpAddr, trusted: &[IpNet]) -> bool {
    let ip = normalize_ip(ip);
    trusted.iter().any(|net| net.contains(&ip))
}

fn default_forwarding_headers() -> Vec<ForwardingHeader> {
    AUTO_FORWARDING_HEADERS
        .into_iter()
        .filter_map(parse_forwarding_header)
        .collect()
}

fn parse_forwarding_headers(raw: &str) -> Vec<ForwardingHeader> {
    let raw = raw.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
        return default_forwarding_headers();
    }

    let mut headers = Vec::new();
    for token in raw.split('|').map(str::trim).filter(|t| !t.is_empty()) {
        if token.eq_ignore_ascii_case("auto") {
            logger::warn(
                "TAWAI_FORWARDED_HEADER cannot mix auto with other headers; using defaults",
            );
            return default_forwarding_headers();
        }
        match parse_forwarding_header(token) {
            Some(header) => headers.push(header),
            None => logger::warn(&format!(
                "Ignoring invalid TAWAI_FORWARDED_HEADER entry: {token}"
            )),
        }
    }

    if headers.is_empty() {
        logger::warn("TAWAI_FORWARDED_HEADER has no valid entries; using defaults");
        return default_forwarding_headers();
    }
    headers
}

fn parse_forwarding_header(name: &str) -> Option<ForwardingHeader> {
    let name = name.trim().to_ascii_lowercase();
    let name = HeaderName::from_bytes(name.as_bytes()).ok()?;
    Some(ForwardingHeader::new(name))
}

fn lookup_client_ip(
    headers: &HeaderMap,
    header: &ForwardingHeader,
    trusted: &[IpNet],
) -> HeaderLookup {
    match header.kind {
        HeaderKind::Single => single_lookup(headers, &header.name),
        HeaderKind::Chain => chain_lookup(headers, &header.name, trusted),
        HeaderKind::Forwarded => forwarded_lookup(headers, &header.name, trusted),
    }
}

fn single_lookup(headers: &HeaderMap, name: &HeaderName) -> HeaderLookup {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return HeaderLookup::Absent;
    };
    if values.next().is_some() {
        return HeaderLookup::Unusable;
    }
    let Ok(value) = value.to_str() else {
        return HeaderLookup::Unusable;
    };
    let value = value.trim();
    if value.is_empty() {
        return HeaderLookup::Absent;
    }
    match parse_ip(value) {
        Some(ip) => HeaderLookup::Ip(ip),
        None => HeaderLookup::Unusable,
    }
}

fn chain_lookup(headers: &HeaderMap, name: &HeaderName, trusted: &[IpNet]) -> HeaderLookup {
    let mut chain = Vec::new();
    for value in headers.get_all(name) {
        let Ok(value) = value.to_str() else {
            return HeaderLookup::Unusable;
        };
        chain.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(parse_ip),
        );
    }
    select_client_ip(&chain, trusted)
}

fn forwarded_lookup(headers: &HeaderMap, name: &HeaderName, trusted: &[IpNet]) -> HeaderLookup {
    let mut chain = Vec::new();
    for value in headers.get_all(name) {
        let Ok(value) = value.to_str() else {
            return HeaderLookup::Unusable;
        };
        if value.trim().is_empty() {
            continue;
        }
        let Ok(forwarded) = ForwardedHeaderValue::from_forwarded(value) else {
            return HeaderLookup::Unusable;
        };
        chain.extend(
            forwarded
                .iter()
                .map(|stanza| stanza.forwarded_for_ip().map(normalize_ip)),
        );
    }
    select_client_ip(&chain, trusted)
}

fn select_client_ip(chain: &[Option<IpAddr>], trusted: &[IpNet]) -> HeaderLookup {
    if chain.is_empty() {
        return HeaderLookup::Absent;
    }
    for entry in chain.iter().rev() {
        match entry {
            Some(ip) if is_trusted(*ip, trusted) => continue,
            Some(ip) => return HeaderLookup::Ip(normalize_ip(*ip)),
            None => return HeaderLookup::Unusable,
        }
    }
    match chain.first().copied().flatten() {
        Some(ip) => HeaderLookup::Ip(normalize_ip(ip)),
        None => HeaderLookup::Unusable,
    }
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
        .map(normalize_ip)
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ip)),
        ip => ip,
    }
}

fn normalize_cidr(net: IpNet) -> IpNet {
    match net {
        IpNet::V6(net) if net.prefix_len() >= 96 => match net.addr().to_ipv4_mapped() {
            Some(addr) => IpNet::new(IpAddr::V4(addr), net.prefix_len() - 96)
                .expect("IPv4-mapped prefix length fits IPv4")
                .trunc(),
            None => IpNet::V6(net),
        },
        net => net,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    fn extractor(trusted: &str) -> TrustedProxyIpKeyExtractor {
        TrustedProxyIpKeyExtractor::from_list(trusted)
    }

    fn extractor_with(trusted: &str, forwarded: &str) -> TrustedProxyIpKeyExtractor {
        TrustedProxyIpKeyExtractor::from_config(trusted, forwarded)
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

    fn key(extractor: &TrustedProxyIpKeyExtractor, request: &Request<Body>) -> ClientKey {
        extractor.extract(request).unwrap()
    }

    fn ip(value: &str) -> ClientKey {
        ClientKey::Ip(value.parse::<IpAddr>().unwrap())
    }

    fn unknown(value: &str) -> ClientKey {
        ClientKey::Unknown(value.parse::<IpAddr>().unwrap())
    }

    fn header_names(headers: &[ForwardingHeader]) -> Vec<&str> {
        headers.iter().map(|header| header.name.as_str()).collect()
    }

    #[test]
    fn untrusted_peer_ignores_forwarded_headers() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "203.0.113.9:1234",
            &[("x-forwarded-for", "1.2.3.4"), ("x-real-ip", "5.6.7.8")],
        );

        assert_eq!(key(&extractor, &request), ip("203.0.113.9"));
    }

    #[test]
    fn trusted_peer_uses_rightmost_untrusted_forwarded_for_ip() {
        let extractor = extractor("10.0.0.0/8");
        let request = request(
            "10.0.0.2:1234",
            &[("x-forwarded-for", "1.2.3.4, 6.6.6.6, 10.0.0.1")],
        );

        assert_eq!(key(&extractor, &request), ip("6.6.6.6"));
    }

    #[test]
    fn trusted_peer_uses_x_real_ip() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[("x-real-ip", "5.6.7.8")]);

        assert_eq!(key(&extractor, &request), ip("5.6.7.8"));
    }

    #[test]
    fn trusted_peer_uses_forwarded_header() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("forwarded", "for=1.2.3.4, for=6.6.6.6")],
        );

        assert_eq!(key(&extractor, &request), ip("6.6.6.6"));
    }

    #[test]
    fn forwarded_unknown_entry_uses_unknown_key() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("forwarded", "for=198.51.100.77, for=unknown")],
        );

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn forwarded_obfuscated_entry_uses_unknown_key() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("forwarded", "for=198.51.100.77, for=_hidden")],
        );

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn malformed_forwarded_header_uses_unknown_key() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[("forwarded", "for=")]);

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn x_forwarded_for_unknown_entry_uses_unknown_key() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("x-forwarded-for", "198.51.100.77, unknown")],
        );

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn x_forwarded_for_skips_empty_segments() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[("x-forwarded-for", "198.51.100.77, ,")]);

        assert_eq!(key(&extractor, &request), ip("198.51.100.77"));
    }

    #[test]
    fn x_real_ip_unparseable_uses_unknown_key() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[("x-real-ip", "unknown")]);

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn x_real_ip_blank_falls_through_to_next_header() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("x-real-ip", " "), ("forwarded", "for=1.2.3.4")],
        );

        assert_eq!(key(&extractor, &request), ip("1.2.3.4"));
    }

    #[test]
    fn multiple_x_real_ip_values_are_unusable() {
        let extractor = extractor("10.0.0.1");
        let request = request(
            "10.0.0.1:1234",
            &[("x-real-ip", "1.2.3.4"), ("x-real-ip", "5.6.7.8")],
        );

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn first_present_unusable_header_stops_resolution() {
        let extractor = extractor_with("10.0.0.1", "x-real-ip|x-forwarded-for");
        let request = request(
            "10.0.0.1:1234",
            &[("x-real-ip", "unknown"), ("x-forwarded-for", "9.9.9.9")],
        );

        assert_eq!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn configured_header_order_is_respected() {
        let extractor = extractor_with("10.0.0.1", "x-real-ip|x-forwarded-for");
        let request = request(
            "10.0.0.1:1234",
            &[("x-real-ip", "5.6.7.8"), ("x-forwarded-for", "9.9.9.9")],
        );

        assert_eq!(key(&extractor, &request), ip("5.6.7.8"));
    }

    #[test]
    fn custom_header_is_parsed_as_chain() {
        let extractor = extractor_with("10.0.0.1", "x-forwarded-ip");
        let request = request("10.0.0.1:1234", &[("x-forwarded-ip", "1.2.3.4, 10.0.0.1")]);

        assert_eq!(key(&extractor, &request), ip("1.2.3.4"));
    }

    #[test]
    fn explicit_forwarded_ignores_other_forwarding_headers() {
        let extractor = extractor_with("10.0.0.1", "forwarded");
        let request = request(
            "10.0.0.1:1234",
            &[
                ("x-forwarded-for", "9.9.9.9"),
                ("x-real-ip", "8.8.8.8"),
                ("forwarded", "for=1.2.3.4"),
            ],
        );

        assert_eq!(key(&extractor, &request), ip("1.2.3.4"));
    }

    #[test]
    fn explicit_x_forwarded_for_ignores_other_forwarding_headers() {
        let extractor = extractor_with("10.0.0.1", "x-forwarded-for");
        let request = request(
            "10.0.0.1:1234",
            &[
                ("x-real-ip", "8.8.8.8"),
                ("forwarded", "for=1.2.3.4"),
                ("x-forwarded-for", "9.9.9.9"),
            ],
        );

        assert_eq!(key(&extractor, &request), ip("9.9.9.9"));
    }

    #[test]
    fn explicit_x_real_ip_ignores_other_forwarding_headers() {
        let extractor = extractor_with("10.0.0.1", "x-real-ip");
        let request = request(
            "10.0.0.1:1234",
            &[
                ("x-forwarded-for", "9.9.9.9"),
                ("forwarded", "for=1.2.3.4"),
                ("x-real-ip", "8.8.8.8"),
            ],
        );

        assert_eq!(key(&extractor, &request), ip("8.8.8.8"));
    }

    #[test]
    fn absent_configured_header_falls_back_to_peer() {
        let extractor = extractor_with("10.0.0.1", "x-real-ip");
        let request = request(
            "10.0.0.1:1234",
            &[("x-forwarded-for", "9.9.9.9"), ("forwarded", "for=1.2.3.4")],
        );

        assert_eq!(key(&extractor, &request), ip("10.0.0.1"));
    }

    #[test]
    fn blank_forwarding_headers_use_peer_key() {
        let extractor = extractor_with("10.0.0.1", "x-forwarded-for|x-real-ip|forwarded");
        let request = request(
            "10.0.0.1:1234",
            &[
                ("x-forwarded-for", ""),
                ("x-real-ip", " "),
                ("forwarded", ""),
            ],
        );

        assert_eq!(key(&extractor, &request), ip("10.0.0.1"));
    }

    #[test]
    fn headerless_request_uses_peer_key() {
        let extractor = extractor("10.0.0.1");
        let request = request("10.0.0.1:1234", &[]);

        assert_eq!(key(&extractor, &request), ip("10.0.0.1"));
        assert_ne!(key(&extractor, &request), unknown("10.0.0.1"));
    }

    #[test]
    fn key_name_distinguishes_unknown_keys() {
        let extractor = extractor("10.0.0.1");
        assert_eq!(
            extractor.key_name(&ip("1.2.3.4")),
            Some("1.2.3.4".to_string())
        );
        assert_eq!(
            extractor.key_name(&unknown("10.0.0.1")),
            Some("unknown:10.0.0.1".to_string())
        );
    }

    #[test]
    fn default_forwarding_headers_preserve_priority() {
        for raw in ["", "auto", " AUTO "] {
            assert_eq!(
                header_names(&parse_forwarding_headers(raw)),
                vec!["x-forwarded-for", "x-real-ip", "forwarded"],
                "{raw}"
            );
        }
    }

    #[test]
    fn parses_single_and_list_forwarding_headers() {
        assert_eq!(
            header_names(&parse_forwarding_headers("x-real-ip")),
            vec!["x-real-ip"]
        );
        assert_eq!(
            header_names(&parse_forwarding_headers(" X-Forwarded-IP | x-real-ip ")),
            vec!["x-forwarded-ip", "x-real-ip"]
        );
    }

    #[test]
    fn invalid_forwarding_header_entries_are_ignored() {
        assert_eq!(
            header_names(&parse_forwarding_headers("bad header|x-real-ip")),
            vec!["x-real-ip"]
        );
        assert_eq!(
            header_names(&parse_forwarding_headers("bad header")),
            vec!["x-forwarded-for", "x-real-ip", "forwarded"]
        );
        assert_eq!(
            header_names(&parse_forwarding_headers("auto|x-real-ip")),
            vec!["x-forwarded-for", "x-real-ip", "forwarded"]
        );
    }

    #[test]
    fn normalizes_ipv4_mapped_ipv6_peer() {
        let extractor = extractor("127.0.0.1");
        let request = request("[::ffff:127.0.0.1]:1234", &[]);

        assert_eq!(key(&extractor, &request), ip("127.0.0.1"));
    }

    #[test]
    fn normalizes_ipv4_mapped_networks() {
        for (config, expected) in [
            ("::ffff:10.0.0.0/96", "0.0.0.0/0"),
            ("::ffff:10.0.0.0/104", "10.0.0.0/8"),
            ("::ffff:10.0.0.1/128", "10.0.0.1/32"),
            ("::ffff:0:0/95", "::ffff:0:0/95"),
            ("2001:db8::/32", "2001:db8::/32"),
        ] {
            assert_eq!(
                normalize_cidr(config.parse::<IpNet>().unwrap()),
                expected.parse::<IpNet>().unwrap(),
                "{config}"
            );
        }
    }

    #[test]
    fn trusted_peer_uses_ipv4_mapped_address_config() {
        let extractor = extractor("::ffff:10.0.0.1");
        let request = request("10.0.0.1:1234", &[("x-real-ip", "5.6.7.8")]);

        assert_eq!(key(&extractor, &request), ip("5.6.7.8"));
    }

    #[test]
    fn trusted_peer_uses_ipv4_mapped_host_config() {
        let extractor = extractor("::ffff:10.0.0.1/128");
        let request = request("10.0.0.1:1234", &[("x-real-ip", "5.6.7.8")]);

        assert_eq!(key(&extractor, &request), ip("5.6.7.8"));
    }

    #[test]
    fn trusted_peer_uses_ipv4_mapped_cidr_config() {
        let extractor = extractor("::ffff:10.0.0.0/104");
        let request = request("10.2.3.4:1234", &[("x-real-ip", "5.6.7.8")]);

        assert_eq!(key(&extractor, &request), ip("5.6.7.8"));
    }

    #[test]
    fn trusts_loopback_by_default() {
        let extractor = extractor("");
        let request = request("127.0.0.1:1234", &[("x-forwarded-for", "6.6.6.6")]);

        assert_eq!(key(&extractor, &request), ip("6.6.6.6"));
    }
}
