use baia_core::config::{
    ConfigFileStore, ConfigSyncError, DnsProvider, PlatformConfig, PowerDnsMode,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn core_loads_platform_yaml_written_by_an_operator() {
    let workspace = test_workspace();
    let config_path = workspace.join("platform.yaml");
    fs::write(
        &config_path,
        [
            "platform:",
            "  publicUrl: https://admin.example.test",
            "  adminHostname: admin.example.test",
            "modules:",
            "  acme:",
            "    enabled: true",
            "  crowdsec:",
            "    enabled: true",
            "  captcha:",
            "    enabled: false",
            "  redis:",
            "    enabled: true",
            "  email:",
            "    enabled: false",
            "  powerdns:",
            "    enabled: true",
            "  cloudflare:",
            "    enabled: false",
            "  metrics:",
            "    enabled: true",
            "  experimental:",
            "    enabled: false",
            "services:",
            "  postgres:",
            "    host: postgres",
            "    port: 5432",
            "  redis:",
            "    host: redis",
            "    port: 6379",
            "  caddyAdminUrl: http://caddy:2019",
            "integrations:",
            "  powerdns:",
            "    mode: integrated",
            "    apiUrl: http://powerdns:8081/api/v1",
            "    apiKeyEnv: BAIA_POWERDNS_API_KEY",
            "  cloudflare:",
            "    apiTokenEnv: BAIA_CLOUDFLARE_API_TOKEN",
            "    automaticDns:",
            "      enabled: true",
            "      defaultProxied: false",
            "      requireDoubleProxyAcknowledgement: true",
            "  crowdsec:",
            "    localApiUrl: http://crowdsec:8080",
            "    apiKeyEnv: BAIA_CROWDSEC_API_KEY",
            "tls:",
            "  acme:",
            "    emailEnv: BAIA_ACME_EMAIL",
            "    http01Enabled: true",
            "    dnsProvider: powerdns",
            "    wildcardEnabled: true",
        ]
        .join("\n"),
    )
    .expect("fixture config must be written");

    let store = ConfigFileStore::new(&config_path);
    let config = store.load().expect("config must load from YAML");

    assert_eq!(config.platform.public_url, "https://admin.example.test");
    assert!(config.modules.crowdsec.enabled);
    assert_eq!(config.integrations.powerdns.mode, PowerDnsMode::Integrated);
    assert_eq!(config.tls.acme.dns_provider, Some(DnsProvider::PowerDns));

    remove_workspace(&workspace);
}

#[test]
fn panel_changes_are_persisted_back_to_platform_yaml() {
    let workspace = test_workspace();
    let config_path = workspace.join("platform.yaml");
    let mut config = PlatformConfig::default();
    config.platform.public_url = "https://panel.example.test".to_string();
    config.modules.cloudflare.enabled = true;
    config.integrations.cloudflare.account_id = Some("account_123".to_string());
    config.tls.acme.dns_provider = Some(DnsProvider::Cloudflare);
    config.tls.acme.wildcard_enabled = true;

    let store = ConfigFileStore::new(&config_path);
    store.save(&config).expect("valid config must be saved");

    let saved = fs::read_to_string(&config_path).expect("saved config must be readable");
    let reloaded = store.load().expect("saved config must reload");

    assert!(saved.contains("publicUrl: https://panel.example.test"));
    assert!(saved.contains("accountId: account_123"));
    assert!(saved.contains("dnsProvider: cloudflare"));
    assert_eq!(reloaded, config);

    remove_workspace(&workspace);
}

#[test]
fn invalid_panel_changes_are_not_written_to_platform_yaml() {
    let workspace = test_workspace();
    let config_path = workspace.join("platform.yaml");
    let store = ConfigFileStore::new(&config_path);
    let original = PlatformConfig::default();
    store.save(&original).expect("initial config must be saved");
    let original_file = fs::read_to_string(&config_path).expect("initial config must be readable");

    let mut invalid = original.clone();
    invalid.modules.powerdns.enabled = true;
    invalid.integrations.powerdns.api_url = None;

    let error = store
        .save(&invalid)
        .expect_err("invalid config must not be persisted");

    assert_eq!(
        error,
        ConfigSyncError::Validation(
            baia_core::config::ConfigValidationError::MissingPowerDnsApiUrl
        )
    );
    assert_eq!(
        fs::read_to_string(&config_path).expect("config must remain readable"),
        original_file
    );

    remove_workspace(&workspace);
}

fn test_workspace() -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be valid")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("baia-config-sync-{id}"));
    fs::create_dir_all(&path).expect("workspace must be created");
    path
}

fn remove_workspace(path: &Path) {
    fs::remove_dir_all(path).expect("workspace must be removed");
}
