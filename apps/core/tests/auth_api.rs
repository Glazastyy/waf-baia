use axum::body::{Body, to_bytes};
use baia_core::server::{ServerConfig, build_router};
use http::header::{COOKIE, SET_COOKIE};
use http::{Method, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn login_creates_hardened_session_and_unlocks_admin_api() {
    let app = build_router(ServerConfig::for_tests("correct-password"));

    let anonymous = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/components")
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            r#"{"username":"admin","password":"correct-password"}"#,
        ))
        .await
        .expect("request must complete");

    assert_eq!(login.status(), StatusCode::OK);
    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .find_map(|value| {
            let raw = value.to_str().expect("cookie must be valid ascii");
            raw.starts_with("baia_session=").then(|| raw.to_string())
        })
        .expect("session cookie must be set");

    assert!(session_cookie.contains("HttpOnly"));
    assert!(session_cookie.contains("Secure"));
    assert!(session_cookie.contains("SameSite=Strict"));
    assert!(session_cookie.contains("Path=/"));

    let body = response_json(login).await;
    let csrf = body["csrfToken"]
        .as_str()
        .expect("csrf token must be returned to authenticated UI");

    assert!(csrf.len() >= 32);

    let authenticated = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/components")
                .header(COOKIE, cookie_pair(&session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(authenticated.status(), StatusCode::OK);
}

#[tokio::test]
async fn authenticated_mutations_require_csrf_token() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            r#"{"username":"admin","password":"correct-password"}"#,
        ))
        .await
        .expect("request must complete");
    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .find_map(|value| {
            let raw = value.to_str().expect("cookie must be valid ascii");
            raw.starts_with("baia_session=").then(|| raw.to_string())
        })
        .expect("session cookie must be set");
    let body = response_json(login).await;
    let csrf = body["csrfToken"].as_str().expect("csrf token must exist");

    let missing_csrf = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/logout")
                .header(COOKIE, cookie_pair(&session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(missing_csrf.status(), StatusCode::FORBIDDEN);

    let logout = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/logout")
                .header(COOKIE, cookie_pair(&session_cookie))
                .header("x-csrf-token", csrf)
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn repeated_invalid_logins_are_locked_without_leaking_account_state() {
    let app = build_router(ServerConfig::for_tests("correct-password"));

    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(json_request(
                Method::POST,
                "/api/auth/login",
                r#"{"username":"admin","password":"wrong-password"}"#,
            ))
            .await
            .expect("request must complete");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers().get(SET_COOKIE), None);
    }

    let locked = app
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            r#"{"username":"admin","password":"correct-password"}"#,
        ))
        .await
        .expect("request must complete");

    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(locked.headers().get(SET_COOKIE), None);
}

#[tokio::test]
async fn authenticated_configuration_returns_loaded_platform_configuration() {
    let mut server_config = ServerConfig::for_tests("correct-password");
    server_config.platform_config.modules.crowdsec.enabled = true;
    server_config.platform_config.modules.powerdns.enabled = true;
    let app = build_router(server_config);
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            r#"{"username":"admin","password":"correct-password"}"#,
        ))
        .await
        .expect("request must complete");
    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .find_map(|value| {
            let raw = value.to_str().expect("cookie must be valid ascii");
            raw.starts_with("baia_session=").then(|| raw.to_string())
        })
        .expect("session cookie must be set");

    let configuration = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/configuration")
                .header(COOKIE, cookie_pair(&session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(configuration.status(), StatusCode::OK);
    let body = response_json(configuration).await;
    assert_eq!(body["modules"]["crowdsec"]["enabled"], true);
    assert_eq!(body["modules"]["powerdns"]["enabled"], true);
    assert_eq!(body["integrations"]["powerdns"]["mode"], "integrated");
    assert_eq!(body["integrations"]["powerdns"]["apiUrlConfigured"], true);
    assert_eq!(body["integrations"]["powerdns"]["apiKeyConfigured"], true);
    assert_eq!(body["integrations"]["crowdsec"]["localApiConfigured"], true);
    assert_eq!(body["integrations"]["crowdsec"]["apiKeyConfigured"], true);
    assert_eq!(body["integrations"]["powerdns"]["apiUrl"], Value::Null);
    assert_eq!(body["integrations"]["powerdns"]["apiKeyEnv"], Value::Null);
    assert_eq!(body["integrations"]["crowdsec"]["localApiUrl"], Value::Null);
    assert_eq!(body["integrations"]["crowdsec"]["apiKeyEnv"], Value::Null);
}

#[tokio::test]
async fn applications_can_be_created_and_listed_without_sample_data() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;

    let empty = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/applications")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(response_json(empty).await["items"], serde_json::json!([]));

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/applications")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"name":"Portal","hostname":"portal.example.com","upstreams":[{"dial":"10.0.0.20:8080"}]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    assert_eq!(created_body["name"], "Portal");
    assert_eq!(created_body["hostname"], "portal.example.com");
    assert_eq!(created_body["enabled"], true);
    assert_eq!(created_body["upstreams"][0]["dial"], "10.0.0.20:8080");

    let listed = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/applications")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    let listed_body = response_json(listed).await;
    assert_eq!(listed_body["items"].as_array().expect("items must be array").len(), 1);
    assert_eq!(listed_body["items"][0]["hostname"], "portal.example.com");
}

#[tokio::test]
async fn applications_reject_invalid_input_without_mutating_state() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;

    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/applications")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"name":"","hostname":"not a hostname","upstreams":[{"dial":""}]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let listed = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/applications")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(response_json(listed).await["items"], serde_json::json!([]));
}

#[tokio::test]
async fn waf_rules_can_be_created_for_existing_applications_and_listed() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;
    let application = create_application(&app, &authenticated).await;
    let application_id = application["id"].as_str().expect("application id must exist");

    let empty = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/waf/rules")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(response_json(empty).await["items"], serde_json::json!([]));

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/waf/rules")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(format!(
                    r#"{{"name":"Block admin paths","applicationId":"{application_id}","priority":10,"action":"block","pathPrefix":"/admin"}}"#
                )))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    assert_eq!(created_body["name"], "Block admin paths");
    assert_eq!(created_body["applicationId"], application_id);
    assert_eq!(created_body["applicationName"], "Portal");
    assert_eq!(created_body["action"], "block");
    assert_eq!(created_body["enabled"], true);

    let listed = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/waf/rules")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    let listed_body = response_json(listed).await;
    assert_eq!(listed_body["items"].as_array().expect("items must be array").len(), 1);
    assert_eq!(listed_body["items"][0]["applicationName"], "Portal");
}

#[tokio::test]
async fn waf_rules_reject_invalid_action_and_missing_application() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;

    let invalid_action = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/waf/rules")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"name":"Invalid","priority":10,"action":"execute","pathPrefix":"/"}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(invalid_action.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let missing_application = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/waf/rules")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"name":"Missing app","applicationId":"missing","priority":10,"action":"block","pathPrefix":"/"}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(missing_application.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let listed = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/waf/rules")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(response_json(listed).await["items"], serde_json::json!([]));
}

#[tokio::test]
async fn dns_records_can_be_created_and_listed_without_sample_data() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;

    let empty_records = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/dns/records")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(empty_records.status(), StatusCode::OK);
    assert_eq!(response_json(empty_records).await["items"], serde_json::json!([]));

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/dns/records")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"zoneName":"example.com","name":"portal.example.com","recordType":"A","content":"10.0.0.20","ttl":300,"proxied":false}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    assert_eq!(created_body["zoneName"], "example.com");
    assert_eq!(created_body["name"], "portal.example.com");
    assert_eq!(created_body["recordType"], "A");
    assert_eq!(created_body["content"], "10.0.0.20");
    assert_eq!(created_body["ttl"], 300);
    assert_eq!(created_body["proxied"], false);

    let listed_records = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/dns/records")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");
    let listed_body = response_json(listed_records).await;
    assert_eq!(listed_body["items"].as_array().expect("items must be array").len(), 1);
    assert_eq!(listed_body["items"][0]["name"], "portal.example.com");

    let listed_zones = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/dns/zones")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");
    let zones_body = response_json(listed_zones).await;
    assert_eq!(zones_body["items"].as_array().expect("items must be array").len(), 1);
    assert_eq!(zones_body["items"][0]["name"], "example.com");
}

#[tokio::test]
async fn dns_records_reject_invalid_input_without_mutating_state() {
    let app = build_router(ServerConfig::for_tests("correct-password"));
    let authenticated = login_session(&app).await;

    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/dns/records")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"zoneName":"bad zone","name":"","recordType":"TXT","content":"","ttl":30,"proxied":false}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let listed = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/dns/records")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .body(Body::empty())
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(response_json(listed).await["items"], serde_json::json!([]));
}

fn json_request(method: Method, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request must build")
}

async fn response_json(response: http::Response<Body>) -> Value {
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body must be readable");
    serde_json::from_slice(&bytes).expect("body must be json")
}

fn cookie_pair(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .expect("cookie must contain name-value")
        .to_string()
}

struct AuthenticatedSession {
    session_cookie: String,
    csrf_token: String,
}

async fn login_session(app: &axum::Router) -> AuthenticatedSession {
    let login = app
        .clone()
        .oneshot(json_request(
            Method::POST,
            "/api/auth/login",
            r#"{"username":"admin","password":"correct-password"}"#,
        ))
        .await
        .expect("request must complete");
    let session_cookie = login
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .find_map(|value| {
            let raw = value.to_str().expect("cookie must be valid ascii");
            raw.starts_with("baia_session=").then(|| raw.to_string())
        })
        .expect("session cookie must be set");
    let body = response_json(login).await;
    let csrf_token = body["csrfToken"]
        .as_str()
        .expect("csrf token must exist")
        .to_string();

    AuthenticatedSession {
        session_cookie,
        csrf_token,
    }
}

async fn create_application(app: &axum::Router, authenticated: &AuthenticatedSession) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/applications")
                .header("content-type", "application/json")
                .header(COOKIE, cookie_pair(&authenticated.session_cookie))
                .header("x-csrf-token", authenticated.csrf_token.as_str())
                .body(Body::from(
                    r#"{"name":"Portal","hostname":"portal.example.com","upstreams":[{"dial":"10.0.0.20:8080"}]}"#,
                ))
                .expect("request must build"),
        )
        .await
        .expect("request must complete");

    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await
}
