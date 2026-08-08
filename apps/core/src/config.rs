use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformConfig {
    pub platform: PlatformSection,
    pub modules: ModuleSection,
    pub services: ServiceSection,
    pub integrations: IntegrationSection,
    pub tls: TlsSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSection {
    pub public_url: String,
    pub admin_hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct ModuleToggle {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSection {
    pub postgres: NetworkService,
    pub redis: NetworkService,
    pub caddy_admin_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkService {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationSection {
    pub powerdns: PowerDnsIntegration,
    pub cloudflare: CloudflareIntegration,
    pub crowdsec: CrowdSecIntegration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerDnsIntegration {
    pub mode: PowerDnsMode,
    pub api_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerDnsMode {
    Integrated,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareIntegration {
    pub api_token_env: Option<String>,
    pub account_id: Option<String>,
    pub automatic_dns: CloudflareAutomaticDnsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareAutomaticDnsConfig {
    pub enabled: bool,
    pub default_proxied: bool,
    pub require_double_proxy_acknowledgement: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrowdSecIntegration {
    pub local_api_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsSection {
    pub acme: AcmeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcmeConfig {
    pub email_env: String,
    pub http01_enabled: bool,
    pub dns_provider: Option<DnsProvider>,
    pub wildcard_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSyncError {
    Read(String),
    Write(String),
    Parse(String),
    Serialize(String),
    Validation(ConfigValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFileStore {
    path: PathBuf,
}

impl ConfigFileStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<PlatformConfig, ConfigSyncError> {
        let raw = fs::read_to_string(&self.path)
            .map_err(|error| ConfigSyncError::Read(error.to_string()))?;
        let config = yaml_serde::from_str::<PlatformConfig>(&raw)
            .map_err(|error| ConfigSyncError::Parse(error.to_string()))?;
        config.validate().map_err(ConfigSyncError::Validation)?;
        Ok(config)
    }

    pub fn save(&self, config: &PlatformConfig) -> Result<(), ConfigSyncError> {
        config.validate().map_err(ConfigSyncError::Validation)?;
        let raw = yaml_serde::to_string(config)
            .map_err(|error| ConfigSyncError::Serialize(error.to_string()))?;
        self.write_atomically(&raw)
    }

    fn write_atomically(&self, raw: &str) -> Result<(), ConfigSyncError> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|error| ConfigSyncError::Write(error.to_string()))?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("platform.yaml");
        let temporary_path = parent.join(format!(".{file_name}.tmp"));
        fs::write(&temporary_path, raw)
            .map_err(|error| ConfigSyncError::Write(error.to_string()))?;
        fs::rename(&temporary_path, &self.path)
            .map_err(|error| ConfigSyncError::Write(error.to_string()))?;
        Ok(())
    }
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
                    automatic_dns: CloudflareAutomaticDnsConfig {
                        enabled: true,
                        default_proxied: false,
                        require_double_proxy_acknowledgement: true,
                    },
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
