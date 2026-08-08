use crate::rules::{RuleAction, SecurityRule};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    pub hostname: String,
    pub routes: Vec<Route>,
    pub rules: Vec<SecurityRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub path_prefix: String,
    pub upstreams: Vec<Upstream>,
    pub balancing: LoadBalancingPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upstream {
    pub dial: String,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadBalancingPolicy {
    RoundRobin,
    LeastConn,
    Random,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaddyConfigError {
    ApplicationWithoutHostname,
    RouteWithoutUpstream,
    UpstreamWithoutDial,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaddyConfig(pub Value);

pub fn build_caddy_config(applications: &[Application]) -> Result<CaddyConfig, CaddyConfigError> {
    let mut routes = Vec::new();
    let mut registered_hosts = Vec::new();

    for application in applications {
        if application.hostname.trim().is_empty() {
            return Err(CaddyConfigError::ApplicationWithoutHostname);
        }

        registered_hosts.push(application.hostname.clone());

        for rule in sorted_rules(&application.rules) {
            routes.push(rule_route(&application.hostname, rule));
        }

        for route in &application.routes {
            if route.upstreams.is_empty() {
                return Err(CaddyConfigError::RouteWithoutUpstream);
            }

            let mut upstreams = Vec::new();
            for upstream in &route.upstreams {
                if upstream.dial.trim().is_empty() {
                    return Err(CaddyConfigError::UpstreamWithoutDial);
                }
                upstreams.push(json!({
                    "dial": upstream.dial,
                    "weight": upstream.weight
                }));
            }

            routes.push(json!({
                "match": [{
                    "host": [application.hostname],
                    "path": [format!("{}*", route.path_prefix)]
                }],
                "handle": [
                    security_headers_handler(),
                    {
                        "handler": "reverse_proxy",
                        "load_balancing": {
                            "selection_policy": {
                                "policy": route.balancing.as_caddy_policy()
                            }
                        },
                        "upstreams": upstreams
                    }
                ]
            }));
        }
    }

    Ok(CaddyConfig(json!({
        "apps": {
            "http": {
                "servers": {
                    "baia": {
                        "listen": [":443"],
                        "routes": routes
                    },
                    "baia_http": {
                        "listen": [":80"],
                        "routes": http_routes(&registered_hosts)
                    }
                }
            }
        }
    })))
}

fn sorted_rules(rules: &[SecurityRule]) -> Vec<&SecurityRule> {
    let mut rules = rules.iter().collect::<Vec<_>>();
    rules.sort_by_key(|rule| rule.priority);
    rules
}

fn rule_route(hostname: &str, rule: &SecurityRule) -> Value {
    let path = rule
        .path_prefix
        .as_ref()
        .map(|value| format!("{value}*"))
        .unwrap_or_else(|| "/*".to_string());

    json!({
        "match": [{
            "host": [hostname],
            "path": [path]
        }],
        "handle": [rule_handler(rule)],
        "metadata": {
            "baia_rule": rule.name,
            "priority": rule.priority
        }
    })
}

fn rule_handler(rule: &SecurityRule) -> Value {
    match rule.action {
        RuleAction::Allow => json!({
            "handler": "headers",
            "response": {
                "set": {
                    "X-Baia-WAF": ["allow"]
                }
            }
        }),
        RuleAction::Block => json!({
            "handler": "static_response",
            "status_code": 403,
            "body": "Request blocked by Baia WAF"
        }),
        RuleAction::Challenge | RuleAction::Captcha => json!({
            "handler": "static_response",
            "status_code": 403,
            "body": "Challenge required"
        }),
        RuleAction::RateLimit => json!({
            "handler": "static_response",
            "status_code": 429,
            "body": "Rate limit exceeded"
        }),
        RuleAction::Redirect => json!({
            "handler": "static_response",
            "status_code": 302
        }),
        RuleAction::AddHeader
        | RuleAction::RemoveHeader
        | RuleAction::Log
        | RuleAction::ApplyRule => json!({
            "handler": "headers",
            "response": {
                "set": {
                    "X-Baia-WAF": [format!("{:?}", rule.action)]
                }
            }
        }),
    }
}

fn http_routes(registered_hosts: &[String]) -> Vec<Value> {
    let mut routes = Vec::new();

    if !registered_hosts.is_empty() {
        routes.push(json!({
            "match": [{
                "host": registered_hosts
            }],
            "handle": [{
                "handler": "static_response",
                "status_code": 308,
                "headers": {
                    "Location": ["https://{http.request.host}{http.request.uri}"]
                }
            }]
        }));
    }

    routes.push(json!({
        "handle": [{
            "handler": "static_response",
            "status_code": 403,
            "headers": {
                "Content-Type": ["text/html; charset=utf-8"]
            },
            "body": direct_origin_block_page()
        }]
    }));

    routes
}

fn direct_origin_block_page() -> &'static str {
    r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Direct origin access is not allowed</title><style>body{margin:0;font-family:system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;background:#f6f7f9;color:#17202a;display:grid;min-height:100vh;place-items:center}.panel{max-width:720px;padding:40px;border-top:4px solid #d33;background:#fff;box-shadow:0 16px 48px rgba(15,23,42,.12)}h1{font-size:28px;margin:0 0 12px}p{font-size:16px;line-height:1.55;margin:0 0 10px}.code{font-size:13px;color:#5b6673;text-transform:uppercase;letter-spacing:.08em}</style></head><body><main class="panel"><div class="code">Baia WAF 403</div><h1>Direct origin access is not allowed</h1><p>This hostname is not registered in Baia WAF or the request reached the origin directly by IP.</p><p>Register the domain in Baia WAF and access it through its configured hostname.</p></main></body></html>"#
}

fn security_headers_handler() -> Value {
    json!({
        "handler": "headers",
        "response": {
            "set": {
                "X-Content-Type-Options": ["nosniff"],
                "X-Frame-Options": ["DENY"],
                "Referrer-Policy": ["strict-origin-when-cross-origin"]
            }
        }
    })
}

impl LoadBalancingPolicy {
    fn as_caddy_policy(&self) -> &'static str {
        match self {
            LoadBalancingPolicy::RoundRobin => "round_robin",
            LoadBalancingPolicy::LeastConn => "least_conn",
            LoadBalancingPolicy::Random => "random",
        }
    }
}
