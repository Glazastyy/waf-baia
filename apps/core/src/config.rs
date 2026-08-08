use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub platform: PlatformSection,
    pub modules: ModuleSection,
    pub services: ServiceSection,
    pub integrations: IntegrationSection,
    pub tls: TlsSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformSection {
    pub public_url: String,
    pub admin_hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSection {
    pub acme: ModuleToggle,
    pub crowdsec: ModuleToggle,
    pub captcha: ModuleToggle,
    pub redis: ModuleToggle,
    pub email: ModuleToggle,
    pub powerdns: ModuleToggle,
    pub cloudflare: ModuleToggle,
    pub metrics: ModuleToggle,
    pub experimental: ModuleToggle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleToggle {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceSection {
    pub postgres: NetworkService,
    pub redis: NetworkService,
    pub caddy_admin_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkService {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationSection {
    pub powerdns: PowerDnsIntegration,
    pub cloudflare: CloudflareIntegration,
    pub crowdsec: CrowdSecIntegration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerDnsIntegration {
    pub mode: PowerDnsMode,
    pub api_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerDnsMode {
    Integrated,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareIntegration {
    pub api_token_env: Option<String>,
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrowdSecIntegration {
    pub local_api_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsSection {
    pub acme: AcmeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcmeConfig {
    pub email_env: String,
    pub http01_enabled: bool,
    pub dns_provider: Option<DnsProvider>,
    pub wildcard_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsProvider {
    PowerDns,
    Cloudflare,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    MissingPowerDnsApiUrl,
    MissingPowerDnsApiKeyEnv,
    MissingCloudflareApiTokenEnv,
    MissingCrowdSecApiUrl,
    MissingCrowdSecApiKeyEnv,
    WildcardAcmeRequiresDnsProvider,
}

impl PlatformConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.modules.powerdns.enabled && self.integrations.powerdns.api_url.is_none() {
            return Err(ConfigValidationError::MissingPowerDnsApiUrl);
        }

        if self.modules.powerdns.enabled && self.integrations.powerdns.api_key_env.is_none() {
            return Err(ConfigValidationError::MissingPowerDnsApiKeyEnv);
        }

        if self.modules.cloudflare.enabled && self.integrations.cloudflare.api_token_env.is_none() {
            return Err(ConfigValidationError::MissingCloudflareApiTokenEnv);
        }

        if self.modules.crowdsec.enabled && self.integrations.crowdsec.local_api_url.is_none() {
            return Err(ConfigValidationError::MissingCrowdSecApiUrl);
        }

        if self.modules.crowdsec.enabled && self.integrations.crowdsec.api_key_env.is_none() {
            return Err(ConfigValidationError::MissingCrowdSecApiKeyEnv);
        }

        if self.tls.acme.wildcard_enabled && self.tls.acme.dns_provider.is_none() {
            return Err(ConfigValidationError::WildcardAcmeRequiresDnsProvider);
        }

        Ok(())
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            platform: PlatformSection {
                public_url: "https://waf.localhost".to_string(),
                admin_hostname: "admin.waf.localhost".to_string(),
            },
            modules: ModuleSection {
                acme: ModuleToggle { enabled: true },
                crowdsec: ModuleToggle { enabled: false },
                captcha: ModuleToggle { enabled: false },
                redis: ModuleToggle { enabled: true },
                email: ModuleToggle { enabled: false },
                powerdns: ModuleToggle { enabled: false },
                cloudflare: ModuleToggle { enabled: false },
                metrics: ModuleToggle { enabled: true },
                experimental: ModuleToggle { enabled: false },
            },
            services: ServiceSection {
                postgres: NetworkService {
                    host: "postgres".to_string(),
                    port: 5432,
                },
                redis: NetworkService {
                    host: "redis".to_string(),
                    port: 6379,
                },
                caddy_admin_url: "http://caddy:2019".to_string(),
            },
            integrations: IntegrationSection {
                powerdns: PowerDnsIntegration {
                    mode: PowerDnsMode::Integrated,
                    api_url: Some("http://powerdns:8081/api/v1".to_string()),
                    api_key_env: Some("BAIA_POWERDNS_API_KEY".to_string()),
                },
                cloudflare: CloudflareIntegration {
                    api_token_env: Some("BAIA_CLOUDFLARE_API_TOKEN".to_string()),
                    account_id: None,
                },
                crowdsec: CrowdSecIntegration {
                    local_api_url: Some("http://crowdsec:8080".to_string()),
                    api_key_env: Some("BAIA_CROWDSEC_API_KEY".to_string()),
                },
            },
            tls: TlsSection {
                acme: AcmeConfig {
                    email_env: "BAIA_ACME_EMAIL".to_string(),
                    http01_enabled: true,
                    dns_provider: None,
                    wildcard_enabled: false,
                },
            },
        }
    }
}
