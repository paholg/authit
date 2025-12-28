use axum::{
    Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect},
    routing::get,
};
use cookie::Cookie;
use dioxus::server::ServerFnError;
use oauth2::{
    AuthUrl, ClientId, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, StandardErrorResponse, TokenUrl, basic::BasicClient,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use types::{SESSION_COOKIE_NAME, UserData, err};

use crate::{CONFIG, ReqwestExt, storage::Session};

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
    pub oauth_client: ConfiguredClient,
    pub pkce_verifiers: Arc<RwLock<HashMap<String, (String, Instant)>>>,
}

impl AuthState {
    pub fn new() -> types::Result<Self> {
        let kanidm_url = &CONFIG.kanidm_url;
        let authit_url = &CONFIG.authit_url;

        let oauth_client = BasicClient::new(ClientId::new(CONFIG.oauth_client_id.clone()))
            .set_auth_uri(AuthUrl::from_url(kanidm_url.join("/ui/oauth2")?))
            .set_token_uri(TokenUrl::from_url(kanidm_url.join("/oauth2/token")?))
            .set_redirect_uri(RedirectUrl::from_url(authit_url.join("/auth/callback")?));

        Ok(Self {
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

    Redirect::to(auth_url.as_str())
}

#[derive(Deserialize)]
struct AuthCallback {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: SecretString,
}

#[derive(Deserialize)]
struct UserInfoResponse {
    sub: String,
    preferred_username: String,
    name: String,
    groups: Vec<String>,
}

async fn callback(
    State(state): State<AuthState>,
    Query(params): Query<AuthCallback>,
) -> Result<impl IntoResponse, ServerFnError> {
    callback_inner(state, params).await.map_err(Into::into)
}

async fn callback_inner(
    state: AuthState,
    params: AuthCallback,
) -> types::Result<impl IntoResponse> {
    // Retrieve and remove the PKCE verifier
    let (verifier_secret, _) = state
        .pkce_verifiers
        .write()
        .await
        .remove(&params.state)
        .ok_or_else(|| err!("missing pkce verifier"))?;

    let pkce_verifier = PkceCodeVerifier::new(verifier_secret);

    // Exchange authorization code for token (public client, no secret)
    let client = reqwest::Client::new();
    let token_url = CONFIG.kanidm_url.join("oauth2/token")?;

    let token_response: TokenResponse = client
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &params.code),
            (
                "redirect_uri",
                CONFIG.authit_url.join("/auth/callback")?.as_str(),
            ),
            ("client_id", &CONFIG.oauth_client_id),
            ("client_secret", CONFIG.oauth_client_secret.expose_secret()),
            ("code_verifier", pkce_verifier.secret()),
        ])
        .try_send()
        .await?;

    // Fetch user info
    let userinfo_url = CONFIG.kanidm_url.join(&format!(
        "oauth2/openid/{}/userinfo",
        CONFIG.oauth_client_id
    ))?;
    let user_info_response: UserInfoResponse = client
        .get(userinfo_url)
        .bearer_auth(token_response.access_token.expose_secret())
        .try_send()
        .await?;

    let user_data = UserData {
        user_id: user_info_response.sub,
        username: user_info_response.preferred_username,
        display_name: user_info_response.name,
        groups: user_info_response.groups,
        access_token: token_response.access_token,
    };

    // Store session server-side and get signed token
    let session = Session::create(user_data).await?;
    let token = session.as_token()?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .secure(true)
        .build();

    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn logout(headers: HeaderMap) -> impl IntoResponse {
    // Try to delete session from DB
    if let Some(cookie_header) = headers.get(axum::http::header::COOKIE)
        && let Ok(cookie_str) = cookie_header.to_str()
    {
        for part in cookie_str.split(';') {
            let part = part.trim();
            if let Some(token) = part.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
                let _ = Session::delete_token(token).await;
            }
        }
    }

    // Clear the session cookie
    let cookie = Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .max_age(cookie::time::Duration::ZERO)
        .build();

    let mut response = Redirect::to("/login").into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    response
}
