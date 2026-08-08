use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    pub id: ComponentId,
    pub name: &'static str,
    pub role: &'static str,
    pub management_surface: ComponentManagementSurface,
    pub apply_mode: ComponentApplyMode,
    pub settings: Vec<ComponentSetting>,
    pub secrets: Vec<ComponentSecret>,
    pub capabilities: Vec<&'static str>,
    pub user_facing_configuration: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentId {
    Core,
    Web,
    Caddy,
    Postgres,
    Redis,
    PowerDns,
    Cloudflare,
    CrowdSec,
    Acme,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentManagementSurface {
    Core,
    ExternalProvider,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentApplyMode {
    HotReload,
    RestartRequired,
    NoRuntimeApply,
    ExternalApi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentSetting {
    pub key: &'static str,
    pub label: &'static str,
    pub value_type: ComponentSettingType,
    pub required: bool,
    pub sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentSettingType {
    Boolean,
    Hostname,
    Url,
    Port,
    SecretReference,
    ProviderChoice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentSecret {
    pub env_key: &'static str,
}

impl ComponentSecret {
    pub const fn new(env_key: &'static str) -> Self {
        Self { env_key }
    }
}

pub fn component_catalog() -> Vec<ComponentDescriptor> {
    vec![
        ComponentDescriptor {
            id: ComponentId::Core,
            name: "Core",
            role: "Control plane API, validation, persistence, audit and jobs",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::HotReload,
            settings: vec![
                setting(
                    "platform.publicUrl",
                    "Public URL",
                    ComponentSettingType::Url,
                    true,
                ),
                setting(
                    "platform.adminHostname",
                    "Admin hostname",
                    ComponentSettingType::Hostname,
                    true,
                ),
            ],
            secrets: Vec::new(),
            capabilities: vec!["api", "rbac", "audit", "jobs", "configuration"],
            user_facing_configuration: vec!["platform", "modules", "integrations", "tls"],
        },
        ComponentDescriptor {
            id: ComponentId::Web,
            name: "Web panel",
            role: "Administrative UI with i18n",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::NoRuntimeApply,
            settings: Vec::new(),
            secrets: Vec::new(),
            capabilities: vec!["dashboard", "i18n", "guided_configuration"],
            user_facing_configuration: vec!["locale"],
        },
        ComponentDescriptor {
            id: ComponentId::Caddy,
            name: "Caddy",
            role: "Reverse proxy, TLS termination, WAF enforcement and routing",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::HotReload,
            settings: vec![setting(
                "services.caddyAdminUrl",
                "Admin API URL",
                ComponentSettingType::Url,
                true,
            )],
            secrets: Vec::new(),
            capabilities: vec![
                "reverse_proxy",
                "tls",
                "headers",
                "rate_limit",
                "crowdsec_bouncer",
                "dynamic_config",
            ],
            user_facing_configuration: vec![
                "applications",
                "upstreams",
                "waf_rules",
                "rate_limits",
            ],
        },
        ComponentDescriptor {
            id: ComponentId::Postgres,
            name: "PostgreSQL",
            role: "Persistent database for platform state",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::RestartRequired,
            settings: vec![
                setting(
                    "services.postgres.host",
                    "Host",
                    ComponentSettingType::Hostname,
                    true,
                ),
                setting(
                    "services.postgres.port",
                    "Port",
                    ComponentSettingType::Port,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("POSTGRES_PASSWORD")],
            capabilities: vec!["persistence", "migrations", "audit_storage"],
            user_facing_configuration: Vec::new(),
        },
        ComponentDescriptor {
            id: ComponentId::Redis,
            name: "Redis",
            role: "Sessions, cache, locks and transient distributed state",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::RestartRequired,
            settings: vec![
                setting(
                    "services.redis.host",
                    "Host",
                    ComponentSettingType::Hostname,
                    true,
                ),
                setting(
                    "services.redis.port",
                    "Port",
                    ComponentSettingType::Port,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("REDIS_PASSWORD")],
            capabilities: vec!["sessions", "locks", "cache", "queues", "rate_limit_state"],
            user_facing_configuration: Vec::new(),
        },
        ComponentDescriptor {
            id: ComponentId::PowerDns,
            name: "PowerDNS",
            role: "Integrated or external authoritative DNS provider",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::ExternalApi,
            settings: vec![
                setting(
                    "integrations.powerdns.mode",
                    "Mode",
                    ComponentSettingType::ProviderChoice,
                    true,
                ),
                setting(
                    "integrations.powerdns.apiUrl",
                    "API URL",
                    ComponentSettingType::Url,
                    true,
                ),
                sensitive_setting(
                    "integrations.powerdns.apiKeyEnv",
                    "API key environment variable",
                    ComponentSettingType::SecretReference,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("BAIA_POWERDNS_API_KEY")],
            capabilities: vec!["zones", "records", "dnssec", "integrated_dns"],
            user_facing_configuration: vec!["dns_zones", "dns_records"],
        },
        ComponentDescriptor {
            id: ComponentId::Cloudflare,
            name: "Cloudflare",
            role: "External DNS provider, proxied records and DNS-01 support",
            management_surface: ComponentManagementSurface::ExternalProvider,
            apply_mode: ComponentApplyMode::ExternalApi,
            settings: vec![
                sensitive_setting(
                    "integrations.cloudflare.apiTokenEnv",
                    "API token environment variable",
                    ComponentSettingType::SecretReference,
                    true,
                ),
                setting(
                    "integrations.cloudflare.automaticDns.enabled",
                    "Automatic DNS",
                    ComponentSettingType::Boolean,
                    true,
                ),
                setting(
                    "integrations.cloudflare.automaticDns.defaultProxied",
                    "Default proxied records",
                    ComponentSettingType::Boolean,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("BAIA_CLOUDFLARE_API_TOKEN")],
            capabilities: vec![
                "zones",
                "a_records",
                "aaaa_records",
                "caa_records",
                "proxied_dns",
            ],
            user_facing_configuration: vec!["dns_zones", "dns_records", "certificates"],
        },
        ComponentDescriptor {
            id: ComponentId::CrowdSec,
            name: "CrowdSec",
            role: "Threat intelligence, decisions and remediation feed",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::ExternalApi,
            settings: vec![
                setting(
                    "integrations.crowdsec.localApiUrl",
                    "Local API URL",
                    ComponentSettingType::Url,
                    true,
                ),
                sensitive_setting(
                    "integrations.crowdsec.apiKeyEnv",
                    "API key environment variable",
                    ComponentSettingType::SecretReference,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("BAIA_CROWDSEC_API_KEY")],
            capabilities: vec!["decisions", "bouncers", "collections", "appsec"],
            user_facing_configuration: vec!["decisions", "allowlists", "blocklists"],
        },
        ComponentDescriptor {
            id: ComponentId::Acme,
            name: "ACME",
            role: "Certificate issuing and renewal policy",
            management_surface: ComponentManagementSurface::Core,
            apply_mode: ComponentApplyMode::HotReload,
            settings: vec![
                sensitive_setting(
                    "tls.acme.emailEnv",
                    "Email environment variable",
                    ComponentSettingType::SecretReference,
                    true,
                ),
                setting(
                    "tls.acme.http01Enabled",
                    "HTTP-01",
                    ComponentSettingType::Boolean,
                    true,
                ),
                setting(
                    "tls.acme.dnsProvider",
                    "DNS provider",
                    ComponentSettingType::ProviderChoice,
                    false,
                ),
                setting(
                    "tls.acme.wildcardEnabled",
                    "Wildcard certificates",
                    ComponentSettingType::Boolean,
                    true,
                ),
            ],
            secrets: vec![ComponentSecret::new("BAIA_ACME_EMAIL")],
            capabilities: vec!["http_01", "dns_01", "wildcard", "caa_planning"],
            user_facing_configuration: vec!["certificates", "caa_records"],
        },
    ]
}

fn setting(
    key: &'static str,
    label: &'static str,
    value_type: ComponentSettingType,
    required: bool,
) -> ComponentSetting {
    ComponentSetting {
        key,
        label,
        value_type,
        required,
        sensitive: false,
    }
}

fn sensitive_setting(
    key: &'static str,
    label: &'static str,
    value_type: ComponentSettingType,
    required: bool,
) -> ComponentSetting {
    ComponentSetting {
        key,
        label,
        value_type,
        required,
        sensitive: true,
    }
}
