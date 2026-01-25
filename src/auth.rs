use crate::server::ApiContext;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Basic;
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

pub(crate) async fn auth_middleware(
    State(state): State<Arc<ApiContext>>,
    TypedHeader(Authorization(creds)): TypedHeader<Authorization<Basic>>,
    mut request: Request,
    next: Next,
) -> impl IntoResponse {
    match authenticate(&state.db, creds.username(), creds.password()).await {
        Ok(key) => {
            if let Some(k) = key {
                // pass target domain to route
                request.extensions_mut().insert(k.domain);
                next.run(request).await
            } else {
                StatusCode::UNAUTHORIZED.into_response()
            }
        }
        Err(e) => (StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    }
}

#[derive(FromRow)]
pub(crate) struct User {
    pub name: String,
    pub password: String,
    pub domain: String,
}

impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("name", &self.name)
            .field("password", &"[redacted]")
            .field("domain", &self.domain)
            .finish()
    }
}

pub(crate) async fn authenticate(
    db: &SqlitePool,
    client: &str,
    api_key: &str,
) -> Result<Option<User>, anyhow::Error> {
    // get_user(db, client, api_key).await.map_err(From::from)
    validate_credentials(db, client, api_key).await
}

async fn validate_credentials(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> anyhow::Result<Option<User>> {
    let clients: Vec<User> = sqlx::query_as("select * from users where name = ?")
        .bind(username)
        .fetch_all(pool)
        .await?;

    if clients.is_empty() {
        return Ok(None);
    }

    for user in clients {
        let expected = PasswordHash::new(&user.password)?;
        if Argon2::default()
            .verify_password(password.as_bytes(), &expected)
            .is_ok()
        {
            return Ok(Some(user));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {

    // todo tests
}
