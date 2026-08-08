#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiRoute {
    pub method: Method,
    pub path: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl ApiRoute {
    pub fn new(method: Method, path: &'static str) -> Self {
        Self { method, path }
    }
}

pub fn admin_api_routes() -> Vec<ApiRoute> {
    vec![
        ApiRoute::new(Method::Get, "/api/health"),
        ApiRoute::new(Method::Post, "/api/auth/login"),
        ApiRoute::new(Method::Post, "/api/auth/change-password"),
        ApiRoute::new(Method::Post, "/api/auth/logout"),
        ApiRoute::new(Method::Get, "/api/components"),
        ApiRoute::new(Method::Get, "/api/configuration"),
        ApiRoute::new(Method::Patch, "/api/configuration"),
        ApiRoute::new(Method::Post, "/api/configuration/apply"),
        ApiRoute::new(Method::Post, "/api/configuration/reload"),
        ApiRoute::new(Method::Get, "/api/users"),
        ApiRoute::new(Method::Post, "/api/users"),
        ApiRoute::new(Method::Get, "/api/applications"),
        ApiRoute::new(Method::Post, "/api/applications"),
        ApiRoute::new(Method::Get, "/api/waf/rules"),
        ApiRoute::new(Method::Post, "/api/waf/rules"),
        ApiRoute::new(Method::Get, "/api/rate-limits"),
        ApiRoute::new(Method::Post, "/api/rate-limits"),
        ApiRoute::new(Method::Get, "/api/dns/zones"),
        ApiRoute::new(Method::Get, "/api/dns/records"),
        ApiRoute::new(Method::Post, "/api/dns/records"),
        ApiRoute::new(Method::Post, "/api/cloudflare/dns/plan"),
        ApiRoute::new(Method::Post, "/api/cloudflare/dns/apply"),
        ApiRoute::new(Method::Get, "/api/cloudflare/acme-cas"),
        ApiRoute::new(Method::Get, "/api/certificates"),
        ApiRoute::new(Method::Post, "/api/certificates"),
        ApiRoute::new(Method::Get, "/api/crowdsec/decisions"),
        ApiRoute::new(Method::Get, "/api/audit/events"),
        ApiRoute::new(Method::Get, "/api/metrics"),
        ApiRoute::new(Method::Post, "/api/caddy/apply"),
    ]
}
