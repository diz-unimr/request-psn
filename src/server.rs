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
    use crate::config::{Gpas, Ttp};
    use axum::http::header;
    use axum_extra::headers::Authorization;
    use axum_extra::headers::authorization::{Basic, Credentials};
    use axum_test::TestServer;
    use httpmock::Method::POST;
    use httpmock::MockServer;
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

    #[sqlx::test(fixtures("users"))]
    async fn test_pseudonym(pool: SqlitePool) {
        // gpas mock
        let ttp_mock = MockServer::start();
        ttp_mock.mock(|when, then| {
            when.method(POST)
                .path("/ttp-fhir/fhir/gpas/$pseudonymizeAllowCreate");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                      "resourceType": "Parameters",
                      "parameter": [
                        {
                          "name": "pseudonym",
                          "part": [
                            {
                              "name": "target",
                              "valueIdentifier": {
                                "system": "https://ths-greifswald.de/gpas",
                                "value": "test"
                              }
                            },
                            {
                              "name": "original",
                              "valueIdentifier": {
                                "system": "https://ths-greifswald.de/gpas",
                                "value": "test"
                              }
                            },
                            {
                              "name": "pseudonym",
                              "valueIdentifier": {
                                "system": "https://ths-greifswald.de/gpas",
                                "value": "test-psn"
                              }
                            }
                          ]
                        }
                      ]
                    }"#,
                );
        });
        let config = AppConfig {
            ttp: Ttp {
                gpas: Gpas {
                    base_url: ttp_mock.base_url(),
                },
                timeout: 5,
            },
            ..Default::default()
        };

        let client = TtpClient::new(&config.ttp).await.unwrap();
        let state = Arc::new(ApiContext { client, db: pool });
        let router = build_router(state);
        // test server
        let server = TestServer::new(router).unwrap();

        // send request
        let basic: Basic = Authorization::basic("project", "test").0;
        let response = server
            .get("/request-psn/test")
            .add_header(header::AUTHORIZATION, basic.encode())
            .await;

        // assert
        response.assert_status_ok();
        response.assert_text("test-psn");
    }
}
