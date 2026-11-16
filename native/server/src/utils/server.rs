use tawai_core as tcore;
use tcore::app_context::{AppContext, OperationSignals};
use tcore::utils;

use crate::utils::dash_locks::DashGenerationLocks;
use crate::utils::ratelimit::RateLimiters;
use crate::utils::security::JwtResponse;
use crate::{
    docs::create_docs_router,
    tawai::{create_tawai_router, system::status::handle_status},
};

use axum::{Router, routing::get};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use time::Duration as TimeDuration;

#[derive(Debug, Clone)]
pub struct AppState {
    pub context: Arc<AppContext>,
    pub rate_limiters: Arc<RateLimiters>,
    pub dash_generation_locks: Arc<DashGenerationLocks>,
}
pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(context: Arc<AppContext>) -> Arc<Self> {
        Arc::new(Self {
            context,
            dash_generation_locks: Arc::new(DashGenerationLocks::new()),
            rate_limiters: Arc::new(RateLimiters::new()),
        })
    }
}

pub fn secure_compare(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub fn build_jwt_cookie(jar: CookieJar, jwt: &JwtResponse) -> CookieJar {
    let jwt_token = Cookie::build(("tawai_jwt", jwt.access_token.clone()))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::seconds(jwt.expires_in as i64))
        .build();
    let csrf_token = Cookie::build(("tawai_csrf", jwt.csrf_token.clone()))
        .path("/")
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::seconds(jwt.expires_in as i64))
        .build();
    jar.add(jwt_token).add(csrf_token)
}

pub fn create_router(state: SharedState) -> Router {
    let tawai_router = create_tawai_router(state.clone());
    let docs_router = create_docs_router(state.clone()).layer(state.rate_limiters.global_layer());

    let status_router = Router::new()
        .route("/api/tawai/system/status", get(handle_status))
        .layer(state.rate_limiters.public_layer());

    Router::new()
        .nest("/api/tawai", tawai_router)
        .merge(docs_router)
        .merge(status_router)
        .with_state(state)
}

#[derive(Debug)]
pub enum ServerExit {
    Restart,
    Shutdown,
    BindFailed(std::io::Error),
}

pub async fn run_server(
    router: Router,
    port: u16,
    operation_signals: Arc<OperationSignals>,
) -> ServerExit {
    if operation_signals.shutdown_requested() {
        return ServerExit::Shutdown;
    }

    let host = env::var("TAWAI_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr: SocketAddr = match host.parse() {
        Ok(ip) => SocketAddr::new(ip, port),
        Err(_) => SocketAddr::from(([127, 0, 0, 1], port)),
    };
    utils::logger::debug(&format!("HTTP server listening on {}", addr));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            utils::logger::error(&format!("Failed to bind HTTP server: {}", e));
            return ServerExit::BindFailed(e);
        }
    };

    let shutdown_signals = operation_signals.clone();
    let serve = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        tokio::select! {
            _ = shutdown_signals.restart_server.notified() => {}
            _ = shutdown_signals.wait_for_shutdown() => {}
        }
    });

    if let Err(e) = serve.await {
        utils::logger::error(&format!("HTTP server error: {}", e));
    }

    if operation_signals.shutdown_requested() {
        utils::logger::debug("Shutdown signal received, stopping HTTP server...");
        ServerExit::Shutdown
    } else {
        ServerExit::Restart
    }
}

pub async fn run_server_loop(state: SharedState) {
    loop {
        let port = {
            let app_cfg = state.context.cfg().await;
            tcore::utils::config::get_path(&app_cfg.value, "server.port")
                .and_then(|v| v.as_u64())
                .unwrap_or(8181) as u16
        };

        let router = create_router(state.clone());

        let operation_signals = state.context.operation_signals.clone();

        match run_server(router, port, operation_signals).await {
            ServerExit::Shutdown => {
                utils::logger::debug("Shutting down application...");
                state.context.shutdown().await;
                break;
            }
            ServerExit::Restart => {
                utils::logger::debug("Restarting HTTP server...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            ServerExit::BindFailed(_) => {
                utils::logger::debug("Retrying HTTP server after bind failure...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
