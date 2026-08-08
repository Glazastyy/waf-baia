use crate::auth::{hash_password, verify_password};
use crate::components::component_catalog;
use crate::config::{ConfigFileStore, PlatformConfig, PowerDnsMode};
use axum::extract::State;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

const SESSION_COOKIE: &str = "baia_session";
const SESSION_TTL_SECONDS: u64 = 8 * 60 * 60;
const MAX_LOGIN_FAILURES: u32 = 5;
const LOGIN_LOCK_SECONDS: u64 = 15 * 60;

#[derive(Clone)]
pub struct ServerConfig {
    pub bind_addr: SocketAddr,
    pub initial_admin_password: String,
    pub secure_cookies: bool,
    pub platform_config: PlatformConfig,
}

#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<String, UserAccount>>>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    throttles: Arc<Mutex<HashMap<String, LoginThrottle>>>,
    secure_cookies: bool,
    platform_config: PlatformConfig,
}

#[derive(Clone)]
struct UserAccount {
    username: String,
    password_hash: String,
    password_change_required: bool,
    disabled: bool,
}

#[derive(Clone)]
struct Session {
    username: String,
    csrf_token: String,
    expires_at: Instant,
}

#[derive(Clone)]
struct LoginThrottle {
    failures: u32,
    locked_until: Option<Instant>,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    user: SessionUser,
    csrf_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    user: SessionUser,
    csrf_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicConfigurationResponse {
    modules: PublicModuleSection,
    integrations: PublicIntegrationSection,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicModuleSection {
    crowdsec: PublicModuleToggle,
    powerdns: PublicModuleToggle,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicModuleToggle {
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicIntegrationSection {
    powerdns: PublicPowerDnsIntegration,
    crowdsec: PublicCrowdSecIntegration,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicPowerDnsIntegration {
    mode: &'static str,
    api_url_configured: bool,
    api_key_configured: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicCrowdSecIntegration {
    local_api_configured: bool,
    api_key_configured: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionUser {
    username: String,
    password_change_required: bool,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let bind_addr = std::env::var("BAIA_CORE_BIND")
            .ok()
            .and_then(|value| value.parse::<SocketAddr>().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 8080)));
        let initial_admin_password = std::env::var("BAIA_INITIAL_ADMIN_PASSWORD")
            .unwrap_or_else(|_| "change-this-initial-admin-password".to_string());
        let platform_config = std::env::var("BAIA_CONFIG_PATH")
            .ok()
            .and_then(|path| ConfigFileStore::new(path).load().ok())
            .unwrap_or_default();

        Self {
            bind_addr,
            initial_admin_password,
            secure_cookies: true,
            platform_config,
        }
    }

    pub fn for_tests(initial_admin_password: &str) -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            initial_admin_password: initial_admin_password.to_string(),
            secure_cookies: true,
            platform_config: PlatformConfig::default(),
        }
    }
}

pub fn build_router(config: ServerConfig) -> Router {
    let mut users = HashMap::new();
    users.insert(
        "admin".to_string(),
        UserAccount {
            username: "admin".to_string(),
            password_hash: hash_password(&config.initial_admin_password),
            password_change_required: true,
            disabled: false,
        },
    );

    let state = AppState {
        users: Arc::new(Mutex::new(users)),
        sessions: Arc::new(Mutex::new(HashMap::new())),
        throttles: Arc::new(Mutex::new(HashMap::new())),
        secure_cookies: config.secure_cookies,
        platform_config: config.platform_config,
    };

    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/login", post(login))
        .route("/api/auth/session", get(session))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/change-password", post(change_password))
        .route("/api/components", get(components))
        .route(
            "/api/configuration",
            get(configuration).patch(configuration_patch),
        )
        .route("/api/configuration/apply", post(authenticated_no_content))
        .route("/api/configuration/reload", post(authenticated_no_content))
        .route(
            "/api/users",
            get(authenticated_json).post(authenticated_no_content),
        )
        .route(
            "/api/applications",
            get(authenticated_json).post(authenticated_no_content),
        )
        .route(
            "/api/waf/rules",
            get(authenticated_json).post(authenticated_no_content),
        )
        .route(
            "/api/rate-limits",
            get(authenticated_json).post(authenticated_no_content),
        )
        .route("/api/dns/zones", get(authenticated_json))
        .route("/api/dns/records", post(authenticated_no_content))
        .route("/api/cloudflare/dns/plan", post(authenticated_json))
        .route("/api/cloudflare/dns/apply", post(authenticated_no_content))
        .route("/api/cloudflare/acme-cas", get(authenticated_json))
        .route("/api/certificates", get(authenticated_json))
        .route("/api/crowdsec/decisions", get(authenticated_json))
        .route("/api/audit/events", get(authenticated_json))
        .route("/api/metrics", get(authenticated_json))
        .route("/api/caddy/apply", post(authenticated_no_content))
        .with_state(state)
}

pub async fn serve(config: ServerConfig) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    axum::serve(listener, build_router(config)).await
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Response {
    let key = throttle_key(&headers, &request.username);

    if is_locked(&state, &key) {
        return error(StatusCode::TOO_MANY_REQUESTS, "Too many login attempts");
    }

    let authenticated = {
        let users = state.users.lock().expect("users lock must not be poisoned");
        users
            .get(&request.username)
            .filter(|user| !user.disabled)
            .filter(|user| verify_password(&request.password, &user.password_hash))
            .cloned()
    };

    let Some(user) = authenticated else {
        record_failure(&state, &key);
        return error(StatusCode::UNAUTHORIZED, "Invalid username or password");
    };

    clear_failure(&state, &key);

    let session_id = random_token(48);
    let csrf_token = random_token(48);
    let session = Session {
        username: user.username.clone(),
        csrf_token: csrf_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(SESSION_TTL_SECONDS),
    };

    state
        .sessions
        .lock()
        .expect("sessions lock must not be poisoned")
        .insert(session_id.clone(), session);

    let mut response = Json(LoginResponse {
        user: SessionUser {
            username: user.username,
            password_change_required: user.password_change_required,
        },
        csrf_token,
    })
    .into_response();
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&session_cookie(&session_id, state.secure_cookies))
            .expect("cookie value must be valid"),
    );
    response
}

async fn session(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    let Some(user) = current_user(&state, &session.username) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    Json(SessionResponse {
        user: SessionUser {
            username: user.username,
            password_change_required: user.password_change_required,
        },
        csrf_token: session.csrf_token,
    })
    .into_response()
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(session_id) = session_cookie_value(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    let Some(session) = require_session_id(&state, &session_id) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    state
        .sessions
        .lock()
        .expect("sessions lock must not be poisoned")
        .remove(&session_id);

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        SET_COOKIE,
        HeaderValue::from_str(&expired_session_cookie(state.secure_cookies))
            .expect("cookie value must be valid"),
    );
    response
}

async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    if !valid_password_policy(&request.new_password) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Password does not satisfy policy",
        );
    }

    let mut users = state.users.lock().expect("users lock must not be poisoned");
    let Some(user) = users.get_mut(&session.username) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !verify_password(&request.current_password, &user.password_hash) {
        return error(StatusCode::UNAUTHORIZED, "Invalid username or password");
    }

    user.password_hash = hash_password(&request.new_password);
    user.password_change_required = false;

    StatusCode::NO_CONTENT.into_response()
}

async fn components(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    Json(component_catalog()).into_response()
}

async fn configuration(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    Json(public_configuration(&state.platform_config)).into_response()
}

async fn configuration_patch(State(state): State<AppState>, headers: HeaderMap) -> Response {
    authenticated_mutation(state, headers)
}

async fn authenticated_no_content(State(state): State<AppState>, headers: HeaderMap) -> Response {
    authenticated_mutation(state, headers)
}

async fn authenticated_json(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    Json(json!({ "items": [] })).into_response()
}

fn authenticated_mutation(state: AppState, headers: HeaderMap) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    StatusCode::NO_CONTENT.into_response()
}

fn public_configuration(config: &PlatformConfig) -> PublicConfigurationResponse {
    PublicConfigurationResponse {
        modules: PublicModuleSection {
            crowdsec: PublicModuleToggle {
                enabled: config.modules.crowdsec.enabled,
            },
            powerdns: PublicModuleToggle {
                enabled: config.modules.powerdns.enabled,
            },
        },
        integrations: PublicIntegrationSection {
            powerdns: PublicPowerDnsIntegration {
                mode: powerdns_mode(&config.integrations.powerdns.mode),
                api_url_configured: config.integrations.powerdns.api_url.is_some(),
                api_key_configured: config.integrations.powerdns.api_key_env.is_some(),
            },
            crowdsec: PublicCrowdSecIntegration {
                local_api_configured: config.integrations.crowdsec.local_api_url.is_some(),
                api_key_configured: config.integrations.crowdsec.api_key_env.is_some(),
            },
        },
    }
}

fn powerdns_mode(mode: &PowerDnsMode) -> &'static str {
    match mode {
        PowerDnsMode::Integrated => "integrated",
        PowerDnsMode::External => "external",
    }
}

fn require_session(state: &AppState, headers: &HeaderMap) -> Option<Session> {
    let session_id = session_cookie_value(headers)?;
    require_session_id(state, &session_id)
}

fn require_session_id(state: &AppState, session_id: &str) -> Option<Session> {
    let mut sessions = state
        .sessions
        .lock()
        .expect("sessions lock must not be poisoned");
    let session = sessions.get(session_id).cloned()?;

    if session.expires_at <= Instant::now() {
        sessions.remove(session_id);
        return None;
    }

    Some(session)
}

fn current_user(state: &AppState, username: &str) -> Option<UserAccount> {
    state
        .users
        .lock()
        .expect("users lock must not be poisoned")
        .get(username)
        .filter(|user| !user.disabled)
        .cloned()
}

fn valid_csrf(headers: &HeaderMap, session: &Session) -> bool {
    headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| constant_time_eq(value.as_bytes(), session.csrf_token.as_bytes()))
}

fn session_cookie_value(headers: &HeaderMap) -> Option<String> {
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|part| {
                let trimmed = part.trim();
                trimmed
                    .strip_prefix(&format!("{SESSION_COOKIE}="))
                    .map(ToString::to_string)
            })
        })
}

fn session_cookie(session_id: &str, secure: bool) -> String {
    hardened_cookie(
        &format!("{SESSION_COOKIE}={session_id}"),
        secure,
        Some(SESSION_TTL_SECONDS),
    )
}

fn expired_session_cookie(secure: bool) -> String {
    hardened_cookie(&format!("{SESSION_COOKIE}="), secure, Some(0))
}

fn hardened_cookie(base: &str, secure: bool, max_age: Option<u64>) -> String {
    let secure_part = if secure { "; Secure" } else { "" };
    let max_age_part = max_age
        .map(|value| format!("; Max-Age={value}"))
        .unwrap_or_default();
    format!("{base}; Path=/; HttpOnly; SameSite=Strict{secure_part}{max_age_part}")
}

fn throttle_key(headers: &HeaderMap, username: &str) -> String {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    format!("{ip}:{}", username.trim().to_lowercase())
}

fn is_locked(state: &AppState, key: &str) -> bool {
    state
        .throttles
        .lock()
        .expect("throttles lock must not be poisoned")
        .get(key)
        .and_then(|throttle| throttle.locked_until)
        .is_some_and(|until| until > Instant::now())
}

fn record_failure(state: &AppState, key: &str) {
    let mut throttles = state
        .throttles
        .lock()
        .expect("throttles lock must not be poisoned");
    let throttle = throttles.entry(key.to_string()).or_insert(LoginThrottle {
        failures: 0,
        locked_until: None,
    });
    throttle.failures += 1;

    if throttle.failures >= MAX_LOGIN_FAILURES {
        throttle.locked_until = Some(Instant::now() + Duration::from_secs(LOGIN_LOCK_SECONDS));
    }
}

fn clear_failure(state: &AppState, key: &str) {
    state
        .throttles
        .lock()
        .expect("throttles lock must not be poisoned")
        .remove(key);
}

fn valid_password_policy(password: &str) -> bool {
    password.len() >= 16
        && password.chars().any(char::is_lowercase)
        && password.chars().any(char::is_uppercase)
        && password.chars().any(|value| value.is_ascii_digit())
        && password.chars().any(|value| !value.is_alphanumeric())
}

fn random_token(length: usize) -> String {
    Alphanumeric.sample_string(&mut rand::rng(), length)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({
            "error": message,
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or_default()
        })),
    )
        .into_response()
}
