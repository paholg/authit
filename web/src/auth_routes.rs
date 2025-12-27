#![cfg(feature = "server")]

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use cookie::{Cookie, SameSite};
use oauth2::{
    AuthUrl, ClientId, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, StandardErrorResponse, TokenUrl, basic::BasicClient,
};
use secrecy::ExposeSecret;
use server::Config;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use types::{SESSION_COOKIE_NAME, UserSession, encode_session};

type ConfiguredClient = oauth2::Client<
    StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>,
    oauth2::StandardTokenIntrospectionResponse<
        oauth2::EmptyExtraTokenFields,
        oauth2::basic::BasicTokenType,
    >,
    oauth2::StandardRevocableToken,
    StandardErrorResponse<oauth2::RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

#[derive(Clone)]
pub struct AuthState {
    pub config: Config,
    pub oauth_client: ConfiguredClient,
    pub pkce_verifiers: Arc<RwLock<HashMap<String, (String, Instant)>>>,
}

impl AuthState {
    pub fn new(config: Config) -> Result<Self, url::ParseError> {
        let kanidm_url = &config.kanidm_url;

        let oauth_client = BasicClient::new(ClientId::new(config.oauth_client_id.clone()))
            .set_auth_uri(AuthUrl::new(format!("{kanidm_url}/ui/oauth2"))?)
            .set_token_uri(TokenUrl::new(format!("{kanidm_url}/oauth2/token"))?)
            .set_redirect_uri(RedirectUrl::new(config.oauth_redirect_uri.clone())?);

        Ok(Self {
            config,
            oauth_client,
            pkce_verifiers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn cleanup_old_verifiers(&self) {
        let mut verifiers = self.pkce_verifiers.write().await;
        let now = Instant::now();
        let ttl = Duration::from_secs(600); // 10 minutes
        verifiers.retain(|_, (_, created)| now.duration_since(*created) < ttl);
    }
}

pub fn auth_router(state: AuthState) -> Router {
    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", get(logout))
        .with_state(state)
}

async fn login(State(state): State<AuthState>) -> impl IntoResponse {
    tracing::info!("Login route hit - starting OAuth flow");

    // Clean up old PKCE verifiers
    state.cleanup_old_verifiers().await;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_token = CsrfToken::new_random();

    // Store verifier with timestamp
    state.pkce_verifiers.write().await.insert(
        csrf_token.secret().clone(),
        (pkce_verifier.secret().clone(), Instant::now()),
    );

    let (auth_url, _csrf) = state
        .oauth_client
        .authorize_url(|| csrf_token)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("groups".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    tracing::info!("Redirecting to OAuth URL: {}", auth_url);
    Redirect::to(auth_url.as_str())
}

#[derive(Debug, serde::Deserialize)]
struct AuthCallback {
    code: String,
    state: String,
}

async fn callback(
    State(state): State<AuthState>,
    Query(params): Query<AuthCallback>,
) -> Result<impl IntoResponse, AuthError> {
    // Retrieve and remove the PKCE verifier
    let (verifier_secret, _) = state
        .pkce_verifiers
        .write()
        .await
        .remove(&params.state)
        .ok_or(AuthError::InvalidState)?;

    let pkce_verifier = PkceCodeVerifier::new(verifier_secret);

    // Exchange authorization code for token (public client, no secret)
    let client = reqwest::Client::new();
    let token_url = format!("{}/oauth2/token", state.config.kanidm_url);
    tracing::info!("Token exchange URL: {}", token_url);
    tracing::info!("Client ID: {}", state.config.oauth_client_id);
    tracing::info!("Redirect URI: {}", state.config.oauth_redirect_uri);

    let token_response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &params.code),
            ("redirect_uri", &state.config.oauth_redirect_uri),
            ("client_id", &state.config.oauth_client_id),
            (
                "client_secret",
                state.config.oauth_client_secret.expose_secret(),
            ),
            ("code_verifier", pkce_verifier.secret()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Token exchange HTTP error: {}", e);
            AuthError::TokenExchange
        })?;

    let status = token_response.status();
    if !status.is_success() {
        let body = token_response.text().await.unwrap_or_default();
        tracing::error!("Token exchange failed ({}): {}", status, body);
        return Err(AuthError::TokenExchange);
    }

    let token_data: serde_json::Value = token_response
        .json()
        .await
        .map_err(|_| AuthError::TokenExchange)?;

    let access_token = token_data["access_token"]
        .as_str()
        .ok_or(AuthError::TokenExchange)?
        .to_string();

    // Fetch user info
    let userinfo_response = client
        .get(format!(
            "{}/oauth2/openid/{}/userinfo",
            state.config.kanidm_url, state.config.oauth_client_id
        ))
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|_| AuthError::UserInfo)?;

    if !userinfo_response.status().is_success() {
        tracing::error!(
            "Userinfo fetch failed: {}",
            userinfo_response.text().await.unwrap_or_default()
        );
        return Err(AuthError::UserInfo);
    }

    let userinfo: serde_json::Value = userinfo_response
        .json()
        .await
        .map_err(|_| AuthError::UserInfo)?;

    tracing::info!("Userinfo response: {:?}", userinfo);
    tracing::info!("Groups from userinfo: {:?}", userinfo["groups"]);

    let user_session = UserSession {
        user_id: userinfo["sub"].as_str().unwrap_or("").to_string(),
        username: userinfo["preferred_username"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        display_name: userinfo["name"].as_str().unwrap_or("").to_string(),
        groups: userinfo["groups"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        access_token: access_token.into(),
    };

    // Encode session and set cookie
    let session_value = encode_session(&user_session).map_err(|_| AuthError::Session)?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, session_value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(false) // Set to true in production with HTTPS
        .build();

    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn logout() -> impl IntoResponse {
    // Clear the session cookie
    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::ZERO)
        .build();

    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    response
}

#[derive(Debug)]
enum AuthError {
    InvalidState,
    TokenExchange,
    UserInfo,
    Session,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidState => (StatusCode::BAD_REQUEST, "Invalid OAuth state"),
            AuthError::TokenExchange => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Token exchange failed")
            }
            AuthError::UserInfo => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch user info",
            ),
            AuthError::Session => (StatusCode::INTERNAL_SERVER_ERROR, "Session error"),
        };
        (status, message).into_response()
    }
}
