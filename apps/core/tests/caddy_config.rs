use baia_core::caddy::{Application, LoadBalancingPolicy, Route, Upstream, build_caddy_config};
use baia_core::rules::{RuleAction, SecurityRule};

#[test]
fn caddy_config_contains_routes_upstreams_and_security_headers() {
    let app = Application {
        hostname: "example.test".to_string(),
        routes: vec![Route {
            path_prefix: "/api".to_string(),
            upstreams: vec![
                Upstream {
                    dial: "app-a:8080".to_string(),
                    weight: 2,
                },
                Upstream {
                    dial: "app-b:8080".to_string(),
                    weight: 1,
                },
            ],
            balancing: LoadBalancingPolicy::RoundRobin,
        }],
        rules: vec![SecurityRule {
            name: "block-admin".to_string(),
            priority: 10,
            path_prefix: Some("/admin".to_string()),
            action: RuleAction::Block,
        }],
    };

    let config = build_caddy_config(&[app]).expect("config must be generated");
    let json = serde_json::to_value(config).expect("config must be serializable");

    assert_eq!(json["apps"]["http"]["servers"]["baia"]["listen"][0], ":443");
    assert_eq!(
        json["apps"]["http"]["servers"]["baia_http"]["listen"][0],
        ":80"
    );
    assert!(json.to_string().contains("app-a:8080"));
    assert!(json.to_string().contains("X-Content-Type-Options"));
    assert!(json.to_string().contains("block-admin"));
    assert!(
        json["apps"]["http"]["servers"]["baia_http"]["routes"]
            .to_string()
            .contains("Direct origin access is not allowed")
    );
    assert!(
        json["apps"]["http"]["servers"]["baia_http"]["routes"]
            .to_string()
            .contains("example.test")
    );
}
