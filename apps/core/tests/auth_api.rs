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
