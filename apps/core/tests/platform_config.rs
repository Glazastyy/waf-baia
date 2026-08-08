use baia_core::config::{ConfigValidationError, PlatformConfig};

#[test]
fn default_config_enables_required_foundation_modules() {
    let config = PlatformConfig::default();

    assert!(config.modules.acme.enabled);
    assert!(config.modules.redis.enabled);
    assert!(config.modules.metrics.enabled);
    assert_eq!(config.services.postgres.host, "postgres");
    assert_eq!(config.services.redis.host, "redis");
}

#[test]
fn validation_rejects_enabled_powerdns_without_api_url() {
    let mut config = PlatformConfig::default();
    config.modules.powerdns.enabled = true;
    config.integrations.powerdns.api_url = None;

    let error = config
        .validate()
        .expect_err("PowerDNS must require an API URL");

    assert_eq!(error, ConfigValidationError::MissingPowerDnsApiUrl);
}

#[test]
fn validation_rejects_wildcard_acme_without_dns_provider() {
    let mut config = PlatformConfig::default();
    config.tls.acme.wildcard_enabled = true;
    config.tls.acme.dns_provider = None;

    let error = config
        .validate()
        .expect_err("Wildcard ACME must require DNS-01 provider");

    assert_eq!(
        error,
        ConfigValidationError::WildcardAcmeRequiresDnsProvider
    );
}
