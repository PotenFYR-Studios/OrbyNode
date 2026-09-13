//! OrbyNode daemon entry point (ADR 001): the daemon is the product runtime.

use std::sync::Arc;

use orbynode_api::{AppState, WebSource};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let cfg = orbynode_core::Config::from_env();
    init_tracing(&cfg);

    std::fs::create_dir_all(&cfg.data_dir)
        .unwrap_or_else(|e| panic!("cannot create data dir {}: {e}", cfg.data_dir.display()));
    let db = orbynode_database::Db::open(&orbynode_core::db_url(&cfg.data_dir))
        .await
        .unwrap_or_else(|e| panic!("cannot open database: {e}"));
    tracing::info!(dir = %cfg.data_dir.display(), "database ready");

    // ADR 021: restore shape before serving requests.
    let engine =
        orbynode_api::workspace_engine::WorkspaceEngine::new(db.clone(), Default::default());
    let report = engine.restore().await;
    tracing::info!(
        workspaces = report.workspaces,
        tabs = report.tabs,
        panes = report.panes,
        degraded = report.degraded,
        resumed_agents = report.resumed_agents,
        "workspace restore complete"
    );

    let terminals =
        orbynode_terminal::TerminalManager::new(orbynode_terminal::TerminalConfig::default());
    let realtime = Arc::new(orbynode_realtime::EventBus::new(
        orbynode_realtime::ReplayConfig::default(),
    ));
    let state = AppState::new(
        cfg.static_dir
            .clone()
            .map_or(WebSource::Embedded, WebSource::Disk),
        terminals,
        realtime,
        db,
    );
    let app = orbynode_api::build_router(state);

    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {}: {e}", cfg.bind));

    tracing::info!(
        name = orbynode_core::NAME,
        version = orbynode_core::version(),
        addr = %cfg.bind,
        "daemon listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

fn init_tracing(cfg: &orbynode_core::Config) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    if cfg.log_json {
        registry
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }
}
