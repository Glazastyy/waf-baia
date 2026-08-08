use baia_core::api::{ApiRoute, Method, admin_api_routes};

#[test]
fn api_contract_exposes_expected_initial_resources() {
    let routes = admin_api_routes();

    assert!(routes.contains(&ApiRoute::new(Method::Get, "/api/health")));
    assert!(routes.contains(&ApiRoute::new(Method::Post, "/api/auth/login")));
    assert!(routes.contains(&ApiRoute::new(Method::Get, "/api/applications")));
    assert!(routes.contains(&ApiRoute::new(Method::Post, "/api/waf/rules")));
    assert!(routes.contains(&ApiRoute::new(Method::Get, "/api/audit/events")));
    assert!(routes.contains(&ApiRoute::new(Method::Post, "/api/caddy/apply")));
}
