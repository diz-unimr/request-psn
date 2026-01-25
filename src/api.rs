use crate::error::ApiError;
use crate::server::ApiContext;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use std::sync::Arc;

pub(crate) fn router() -> Router<Arc<ApiContext>> {
    Router::new().route("/request-psn/{original}", get(pseudonym))
}

async fn pseudonym(
    State(ctx): State<Arc<ApiContext>>,
    Extension(domain): Extension<String>,
    Path(original): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let psn = ctx.client.pseudonymize(domain, original).await?;

    Ok((StatusCode::OK, psn))
}
