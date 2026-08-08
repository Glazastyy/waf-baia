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
    applications: Arc<Mutex<Vec<ApplicationRecord>>>,
    waf_rules: Arc<Mutex<Vec<WafRuleRecord>>>,
    dns_zones: Arc<Mutex<Vec<DnsZoneRecord>>>,
    dns_records: Arc<Mutex<Vec<DnsRecordRecord>>>,
    certificates: Arc<Mutex<Vec<CertificateRecord>>>,
    audit_events: Arc<Mutex<Vec<AuditEventRecord>>>,
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

#[derive(Clone)]
struct ApplicationRecord {
    id: String,
    name: String,
    hostname: String,
    enabled: bool,
    upstreams: Vec<UpstreamRecord>,
}

#[derive(Clone)]
struct UpstreamRecord {
    id: String,
    dial: String,
    weight: u32,
    enabled: bool,
}

#[derive(Clone)]
struct WafRuleRecord {
    id: String,
    name: String,
    application_id: Option<String>,
    application_name: Option<String>,
    priority: u32,
    action: WafRuleAction,
    path_prefix: Option<String>,
    enabled: bool,
}

#[derive(Clone)]
enum WafRuleAction {
    Allow,
    Block,
    Challenge,
    RateLimit,
    Log,
}

#[derive(Clone)]
struct DnsZoneRecord {
    id: String,
    provider: String,
    name: String,
}

#[derive(Clone)]
struct DnsRecordRecord {
    id: String,
    zone_id: String,
    zone_name: String,
    name: String,
    record_type: DnsRecordType,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Clone)]
enum DnsRecordType {
    A,
    Aaaa,
    Cname,
    Txt,
    Caa,
    Mx,
}

#[derive(Clone)]
struct CertificateRecord {
    id: String,
    application_id: Option<String>,
    application_name: Option<String>,
    domain: String,
    issuer: String,
    challenge_type: CertificateChallengeType,
    status: CertificateStatus,
}

#[derive(Clone)]
enum CertificateChallengeType {
    Http01,
    Dns01,
}

#[derive(Clone)]
enum CertificateStatus {
    Pending,
    Issued,
    Failed,
    Revoked,
}

#[derive(Clone)]
struct AuditEventRecord {
    id: String,
    actor: String,
    action: &'static str,
    resource_type: &'static str,
    resource_id: String,
    result: &'static str,
    occurred_at: String,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateApplicationRequest {
    name: String,
    hostname: String,
    upstreams: Vec<CreateUpstreamRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUpstreamRequest {
    dial: String,
    weight: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateWafRuleRequest {
    name: String,
    application_id: Option<String>,
    priority: u32,
    action: String,
    path_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDnsRecordRequest {
    zone_name: String,
    name: String,
    record_type: String,
    content: String,
    ttl: u32,
    proxied: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCertificateRequest {
    application_id: Option<String>,
    domain: String,
    issuer: String,
    challenge_type: String,
    status: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationListResponse {
    items: Vec<ApplicationResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationResponse {
    id: String,
    name: String,
    hostname: String,
    enabled: bool,
    upstreams: Vec<UpstreamResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpstreamResponse {
    id: String,
    dial: String,
    weight: u32,
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WafRuleListResponse {
    items: Vec<WafRuleResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WafRuleResponse {
    id: String,
    name: String,
    application_id: Option<String>,
    application_name: Option<String>,
    priority: u32,
    action: &'static str,
    path_prefix: Option<String>,
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DnsZoneListResponse {
    items: Vec<DnsZoneResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DnsZoneResponse {
    id: String,
    provider: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DnsRecordListResponse {
    items: Vec<DnsRecordResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DnsRecordResponse {
    id: String,
    zone_id: String,
    zone_name: String,
    name: String,
    record_type: &'static str,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CertificateListResponse {
    items: Vec<CertificateResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CertificateResponse {
    id: String,
    application_id: Option<String>,
    application_name: Option<String>,
    domain: String,
    issuer: String,
    challenge_type: &'static str,
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditEventListResponse {
    items: Vec<AuditEventResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditEventResponse {
    id: String,
    actor: String,
    action: &'static str,
    resource_type: &'static str,
    resource_id: String,
    result: &'static str,
    occurred_at: String,
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
        applications: Arc::new(Mutex::new(Vec::new())),
        waf_rules: Arc::new(Mutex::new(Vec::new())),
        dns_zones: Arc::new(Mutex::new(Vec::new())),
        dns_records: Arc::new(Mutex::new(Vec::new())),
        certificates: Arc::new(Mutex::new(Vec::new())),
        audit_events: Arc::new(Mutex::new(Vec::new())),
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
            get(applications_index).post(applications_create),
        )
        .route(
            "/api/waf/rules",
            get(waf_rules_index).post(waf_rules_create),
        )
        .route(
            "/api/rate-limits",
            get(authenticated_json).post(authenticated_no_content),
        )
        .route("/api/dns/zones", get(dns_zones_index))
        .route("/api/dns/records", get(dns_records_index).post(dns_records_create))
        .route("/api/cloudflare/dns/plan", post(authenticated_json))
        .route("/api/cloudflare/dns/apply", post(authenticated_no_content))
        .route("/api/cloudflare/acme-cas", get(authenticated_json))
        .route("/api/certificates", get(certificates_index).post(certificates_create))
        .route("/api/crowdsec/decisions", get(authenticated_json))
        .route("/api/audit/events", get(audit_events_index))
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

async fn applications_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let applications = state
        .applications
        .lock()
        .expect("applications lock must not be poisoned")
        .iter()
        .cloned()
        .map(application_response)
        .collect();

    Json(ApplicationListResponse {
        items: applications,
    })
    .into_response()
}

async fn applications_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateApplicationRequest>,
) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    let Ok(application) = application_from_request(request) else {
        return error(StatusCode::UNPROCESSABLE_ENTITY, "Application input is invalid");
    };

    let mut applications = state
        .applications
        .lock()
        .expect("applications lock must not be poisoned");

    if applications
        .iter()
        .any(|existing| existing.hostname.eq_ignore_ascii_case(&application.hostname))
    {
        return error(StatusCode::CONFLICT, "Application hostname already exists");
    }

    applications.push(application.clone());
    record_audit_event(
        &state,
        &session.username,
        "application.create",
        "application",
        &application.id,
        "success",
    );

    (StatusCode::CREATED, Json(application_response(application))).into_response()
}

async fn waf_rules_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let rules = state
        .waf_rules
        .lock()
        .expect("waf rules lock must not be poisoned")
        .iter()
        .cloned()
        .map(waf_rule_response)
        .collect();

    Json(WafRuleListResponse { items: rules }).into_response()
}

async fn waf_rules_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateWafRuleRequest>,
) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    let applications = state
        .applications
        .lock()
        .expect("applications lock must not be poisoned");
    let Ok(rule) = waf_rule_from_request(request, &applications) else {
        return error(StatusCode::UNPROCESSABLE_ENTITY, "WAF rule input is invalid");
    };
    drop(applications);

    state
        .waf_rules
        .lock()
        .expect("waf rules lock must not be poisoned")
        .push(rule.clone());
    record_audit_event(
        &state,
        &session.username,
        "waf_rule.create",
        "waf_rule",
        &rule.id,
        "success",
    );

    (StatusCode::CREATED, Json(waf_rule_response(rule))).into_response()
}

async fn dns_zones_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let zones = state
        .dns_zones
        .lock()
        .expect("dns zones lock must not be poisoned")
        .iter()
        .cloned()
        .map(dns_zone_response)
        .collect();

    Json(DnsZoneListResponse { items: zones }).into_response()
}

async fn dns_records_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let records = state
        .dns_records
        .lock()
        .expect("dns records lock must not be poisoned")
        .iter()
        .cloned()
        .map(dns_record_response)
        .collect();

    Json(DnsRecordListResponse { items: records }).into_response()
}

async fn dns_records_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateDnsRecordRequest>,
) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    let Ok(mut record) = dns_record_from_request(request) else {
        return error(StatusCode::UNPROCESSABLE_ENTITY, "DNS record input is invalid");
    };

    let mut zones = state
        .dns_zones
        .lock()
        .expect("dns zones lock must not be poisoned");
    let zone = ensure_dns_zone(&mut zones, &record.zone_name);
    record.zone_id = zone.id.clone();
    record.zone_name = zone.name.clone();
    drop(zones);

    state
        .dns_records
        .lock()
        .expect("dns records lock must not be poisoned")
        .push(record.clone());
    record_audit_event(
        &state,
        &session.username,
        "dns_record.create",
        "dns_record",
        &record.id,
        "success",
    );

    (StatusCode::CREATED, Json(dns_record_response(record))).into_response()
}

async fn certificates_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let certificates = state
        .certificates
        .lock()
        .expect("certificates lock must not be poisoned")
        .iter()
        .cloned()
        .map(certificate_response)
        .collect();

    Json(CertificateListResponse {
        items: certificates,
    })
    .into_response()
}

async fn certificates_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateCertificateRequest>,
) -> Response {
    let Some(session) = require_session(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    if !valid_csrf(&headers, &session) {
        return error(StatusCode::FORBIDDEN, "CSRF token is invalid");
    }

    let applications = state
        .applications
        .lock()
        .expect("applications lock must not be poisoned");
    let Ok(certificate) = certificate_from_request(request, &applications) else {
        return error(StatusCode::UNPROCESSABLE_ENTITY, "Certificate input is invalid");
    };
    drop(applications);

    state
        .certificates
        .lock()
        .expect("certificates lock must not be poisoned")
        .push(certificate.clone());
    record_audit_event(
        &state,
        &session.username,
        "certificate.create",
        "certificate",
        &certificate.id,
        "success",
    );

    (StatusCode::CREATED, Json(certificate_response(certificate))).into_response()
}

async fn audit_events_index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if require_session(&state, &headers).is_none() {
        return error(StatusCode::UNAUTHORIZED, "Authentication required");
    }

    let events = state
        .audit_events
        .lock()
        .expect("audit events lock must not be poisoned")
        .iter()
        .cloned()
        .map(audit_event_response)
        .collect();

    Json(AuditEventListResponse { items: events }).into_response()
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

fn application_from_request(
    request: CreateApplicationRequest,
) -> Result<ApplicationRecord, ApplicationValidationError> {
    let name = normalized_name(&request.name).ok_or(ApplicationValidationError)?;
    let hostname = normalized_hostname(&request.hostname).ok_or(ApplicationValidationError)?;

    if request.upstreams.is_empty() || request.upstreams.len() > 16 {
        return Err(ApplicationValidationError);
    }

    let upstreams = request
        .upstreams
        .into_iter()
        .map(upstream_from_request)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ApplicationRecord {
        id: random_token(24),
        name,
        hostname,
        enabled: true,
        upstreams,
    })
}

fn upstream_from_request(
    request: CreateUpstreamRequest,
) -> Result<UpstreamRecord, ApplicationValidationError> {
    let dial = normalized_dial(&request.dial).ok_or(ApplicationValidationError)?;
    let weight = request.weight.unwrap_or(1);

    if weight == 0 || weight > 1000 {
        return Err(ApplicationValidationError);
    }

    Ok(UpstreamRecord {
        id: random_token(24),
        dial,
        weight,
        enabled: true,
    })
}

fn application_response(application: ApplicationRecord) -> ApplicationResponse {
    ApplicationResponse {
        id: application.id,
        name: application.name,
        hostname: application.hostname,
        enabled: application.enabled,
        upstreams: application
            .upstreams
            .into_iter()
            .map(|upstream| UpstreamResponse {
                id: upstream.id,
                dial: upstream.dial,
                weight: upstream.weight,
                enabled: upstream.enabled,
            })
            .collect(),
    }
}

fn waf_rule_from_request(
    request: CreateWafRuleRequest,
    applications: &[ApplicationRecord],
) -> Result<WafRuleRecord, WafRuleValidationError> {
    let name = normalized_name(&request.name).ok_or(WafRuleValidationError)?;
    let action = waf_rule_action(&request.action).ok_or(WafRuleValidationError)?;
    let path_prefix = match request.path_prefix.as_deref() {
        Some(value) => normalized_path_prefix(value)?,
        None => None,
    };
    let (application_id, application_name) =
        resolve_rule_application(request.application_id, applications)?;

    if request.priority > 100_000 {
        return Err(WafRuleValidationError);
    }

    Ok(WafRuleRecord {
        id: random_token(24),
        name,
        application_id,
        application_name,
        priority: request.priority,
        action,
        path_prefix,
        enabled: true,
    })
}

fn resolve_rule_application(
    application_id: Option<String>,
    applications: &[ApplicationRecord],
) -> Result<(Option<String>, Option<String>), WafRuleValidationError> {
    let Some(application_id) = application_id else {
        return Ok((None, None));
    };

    let normalized = application_id.trim();

    if normalized.is_empty() {
        return Err(WafRuleValidationError);
    }

    applications
        .iter()
        .find(|application| application.id == normalized)
        .map(|application| {
            (
                Some(application.id.clone()),
                Some(application.name.clone()),
            )
        })
        .ok_or(WafRuleValidationError)
}

fn waf_rule_response(rule: WafRuleRecord) -> WafRuleResponse {
    WafRuleResponse {
        id: rule.id,
        name: rule.name,
        application_id: rule.application_id,
        application_name: rule.application_name,
        priority: rule.priority,
        action: rule.action.as_str(),
        path_prefix: rule.path_prefix,
        enabled: rule.enabled,
    }
}

fn waf_rule_action(value: &str) -> Option<WafRuleAction> {
    match value.trim().to_ascii_lowercase().as_str() {
        "allow" => Some(WafRuleAction::Allow),
        "block" => Some(WafRuleAction::Block),
        "challenge" => Some(WafRuleAction::Challenge),
        "rate_limit" | "rate-limit" => Some(WafRuleAction::RateLimit),
        "log" => Some(WafRuleAction::Log),
        _ => None,
    }
}

impl WafRuleAction {
    fn as_str(&self) -> &'static str {
        match self {
            WafRuleAction::Allow => "allow",
            WafRuleAction::Block => "block",
            WafRuleAction::Challenge => "challenge",
            WafRuleAction::RateLimit => "rate_limit",
            WafRuleAction::Log => "log",
        }
    }
}

fn dns_record_from_request(
    request: CreateDnsRecordRequest,
) -> Result<DnsRecordRecord, DnsRecordValidationError> {
    let zone_name = normalized_hostname(&request.zone_name).ok_or(DnsRecordValidationError)?;
    let name = normalized_hostname(&request.name).ok_or(DnsRecordValidationError)?;
    let record_type = dns_record_type(&request.record_type).ok_or(DnsRecordValidationError)?;
    let content = normalized_dns_content(&request.content).ok_or(DnsRecordValidationError)?;

    if request.ttl < 60 || request.ttl > 86_400 {
        return Err(DnsRecordValidationError);
    }

    Ok(DnsRecordRecord {
        id: random_token(24),
        zone_id: String::new(),
        zone_name,
        name,
        record_type,
        content,
        ttl: request.ttl,
        proxied: request.proxied.unwrap_or(false),
    })
}

fn ensure_dns_zone(zones: &mut Vec<DnsZoneRecord>, zone_name: &str) -> DnsZoneRecord {
    if let Some(zone) = zones.iter().find(|zone| zone.name == zone_name) {
        return zone.clone();
    }

    let zone = DnsZoneRecord {
        id: random_token(24),
        provider: "powerdns".to_string(),
        name: zone_name.to_string(),
    };
    zones.push(zone.clone());
    zone
}

fn dns_zone_response(zone: DnsZoneRecord) -> DnsZoneResponse {
    DnsZoneResponse {
        id: zone.id,
        provider: zone.provider,
        name: zone.name,
    }
}

fn dns_record_response(record: DnsRecordRecord) -> DnsRecordResponse {
    DnsRecordResponse {
        id: record.id,
        zone_id: record.zone_id,
        zone_name: record.zone_name,
        name: record.name,
        record_type: record.record_type.as_str(),
        content: record.content,
        ttl: record.ttl,
        proxied: record.proxied,
    }
}

fn dns_record_type(value: &str) -> Option<DnsRecordType> {
    match value.trim().to_ascii_uppercase().as_str() {
        "A" => Some(DnsRecordType::A),
        "AAAA" => Some(DnsRecordType::Aaaa),
        "CNAME" => Some(DnsRecordType::Cname),
        "TXT" => Some(DnsRecordType::Txt),
        "CAA" => Some(DnsRecordType::Caa),
        "MX" => Some(DnsRecordType::Mx),
        _ => None,
    }
}

impl DnsRecordType {
    fn as_str(&self) -> &'static str {
        match self {
            DnsRecordType::A => "A",
            DnsRecordType::Aaaa => "AAAA",
            DnsRecordType::Cname => "CNAME",
            DnsRecordType::Txt => "TXT",
            DnsRecordType::Caa => "CAA",
            DnsRecordType::Mx => "MX",
        }
    }
}

fn certificate_from_request(
    request: CreateCertificateRequest,
    applications: &[ApplicationRecord],
) -> Result<CertificateRecord, CertificateValidationError> {
    let domain = normalized_hostname(&request.domain).ok_or(CertificateValidationError)?;
    let issuer = normalized_name(&request.issuer).ok_or(CertificateValidationError)?;
    let challenge_type =
        certificate_challenge_type(&request.challenge_type).ok_or(CertificateValidationError)?;
    let status = certificate_status(&request.status).ok_or(CertificateValidationError)?;
    let (application_id, application_name) =
        resolve_rule_application(request.application_id, applications)
            .map_err(|_| CertificateValidationError)?;

    Ok(CertificateRecord {
        id: random_token(24),
        application_id,
        application_name,
        domain,
        issuer,
        challenge_type,
        status,
    })
}

fn certificate_response(certificate: CertificateRecord) -> CertificateResponse {
    CertificateResponse {
        id: certificate.id,
        application_id: certificate.application_id,
        application_name: certificate.application_name,
        domain: certificate.domain,
        issuer: certificate.issuer,
        challenge_type: certificate.challenge_type.as_str(),
        status: certificate.status.as_str(),
    }
}

fn certificate_challenge_type(value: &str) -> Option<CertificateChallengeType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "http_01" | "http-01" => Some(CertificateChallengeType::Http01),
        "dns_01" | "dns-01" => Some(CertificateChallengeType::Dns01),
        _ => None,
    }
}

impl CertificateChallengeType {
    fn as_str(&self) -> &'static str {
        match self {
            CertificateChallengeType::Http01 => "http_01",
            CertificateChallengeType::Dns01 => "dns_01",
        }
    }
}

fn certificate_status(value: &str) -> Option<CertificateStatus> {
    match value.trim().to_ascii_lowercase().as_str() {
        "pending" => Some(CertificateStatus::Pending),
        "issued" => Some(CertificateStatus::Issued),
        "failed" => Some(CertificateStatus::Failed),
        "revoked" => Some(CertificateStatus::Revoked),
        _ => None,
    }
}

impl CertificateStatus {
    fn as_str(&self) -> &'static str {
        match self {
            CertificateStatus::Pending => "pending",
            CertificateStatus::Issued => "issued",
            CertificateStatus::Failed => "failed",
            CertificateStatus::Revoked => "revoked",
        }
    }
}

fn record_audit_event(
    state: &AppState,
    actor: &str,
    action: &'static str,
    resource_type: &'static str,
    resource_id: &str,
    result: &'static str,
) {
    state
        .audit_events
        .lock()
        .expect("audit events lock must not be poisoned")
        .push(AuditEventRecord {
            id: random_token(24),
            actor: actor.to_string(),
            action,
            resource_type,
            resource_id: resource_id.to_string(),
            result,
            occurred_at: unix_timestamp_string(),
        });
}

fn audit_event_response(event: AuditEventRecord) -> AuditEventResponse {
    AuditEventResponse {
        id: event.id,
        actor: event.actor,
        action: event.action,
        resource_type: event.resource_type,
        resource_id: event.resource_id,
        result: event.result,
        occurred_at: event.occurred_at,
    }
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn normalized_name(value: &str) -> Option<String> {
    let normalized = value.trim();

    if normalized.is_empty() || normalized.len() > 120 {
        return None;
    }

    Some(normalized.to_string())
}

fn normalized_hostname(value: &str) -> Option<String> {
    let hostname = value.trim().trim_end_matches('.').to_ascii_lowercase();

    if hostname.len() > 253 || hostname.split('.').count() < 2 {
        return None;
    }

    let valid = hostname.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    });

    valid.then_some(hostname)
}

fn normalized_dial(value: &str) -> Option<String> {
    let dial = value.trim();

    if dial.is_empty() || dial.len() > 255 || dial.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return None;
    }

    Some(dial.to_string())
}

fn normalized_path_prefix(value: &str) -> Result<Option<String>, WafRuleValidationError> {
    let path = value.trim();

    if path.is_empty() {
        return Ok(None);
    }

    if !path.starts_with('/')
        || path.len() > 256
        || path.bytes().any(|byte| byte.is_ascii_control())
        || path.contains("..")
    {
        return Err(WafRuleValidationError);
    }

    Ok(Some(path.to_string()))
}

fn normalized_dns_content(value: &str) -> Option<String> {
    let content = value.trim();

    if content.is_empty() || content.len() > 2048 || content.bytes().any(|byte| byte.is_ascii_control()) {
        return None;
    }

    Some(content.to_string())
}

#[derive(Debug, Clone, Copy)]
struct ApplicationValidationError;

#[derive(Debug, Clone, Copy)]
struct WafRuleValidationError;

#[derive(Debug, Clone, Copy)]
struct DnsRecordValidationError;

#[derive(Debug, Clone, Copy)]
struct CertificateValidationError;

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
