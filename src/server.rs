use crate::api;
use crate::auth::auth_middleware;
use crate::client::TtpClient;
use crate::config::AppConfig;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router, middleware};
use log::debug;
use serde_json::json;
use sqlx::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub(crate) struct ApiContext {
    pub(crate) client: TtpClient,
    pub(crate) db: SqlitePool,
}

pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let filter = format!(
        "{}={level},auth={level},tower_http={level}",
        env!("CARGO_CRATE_NAME"),
        level = config.log_level
    );
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .init();

    let db = SqlitePool::connect("sqlite://db.sqlite?mode=rwc").await?;
    sqlx::migrate!().run(&db).await?;

    let client = TtpClient::new(&config.ttp).await?;
    client.test_connection().await?;

    // context
    let state = Arc::new(ApiContext {
        db: db.clone(),
        client,
    });

    let router = build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    debug!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| e.into())
}

fn build_router(state: Arc<ApiContext>) -> Router {
    Router::new()
        .merge(api::router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route("/status", get(status))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn status(State(ctx): State<Arc<ApiContext>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "healthy": ctx.client.is_healthy().await
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;
    use std::sync::Arc;

    #[sqlx::test]
    async fn status_test(pool: SqlitePool) {
        let config = AppConfig::default();
        let state = Arc::new(ApiContext {
            client: TtpClient::new(&config.ttp).await.unwrap(),
            db: pool,
        });

        // test server
        let router = build_router(state.clone());
        let server = TestServer::new(router).unwrap();

        // send request
        let response = server.get("/status").await;

        // assert
        response.assert_status_ok();
        response.assert_json(&json!({
            "healthy": false,
        }));
    }
}
