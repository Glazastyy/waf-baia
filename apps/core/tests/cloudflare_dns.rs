use baia_core::cloudflare::{
    AcmeIssuer, CloudflareDnsPlanWarning, CloudflareDnsRecordType, DomainOnboardingRequest,
    build_cloudflare_dns_plan, build_cloudflare_record_create_requests, identify_known_acme_ca,
    known_acme_cas,
};
use std::net::{Ipv4Addr, Ipv6Addr};

#[test]
fn cloudflare_plan_creates_a_and_aaaa_records_with_selected_proxy_mode() {
    let request = DomainOnboardingRequest {
        hostname: "app.example.test".to_string(),
        ipv4_addresses: vec![
            Ipv4Addr::new(203, 0, 113, 10),
            Ipv4Addr::new(203, 0, 113, 11),
        ],
        ipv6_addresses: vec![Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 10)],
        cloudflare_proxied: false,
        baia_proxy_enabled: true,
        acme_issuer: AcmeIssuer::LetsEncrypt,
        wildcard_certificate: false,
    };

    let plan = build_cloudflare_dns_plan(request).expect("DNS plan must be valid");

    assert_eq!(plan.records.len(), 3);
    assert_eq!(plan.records[0].record_type, CloudflareDnsRecordType::A);
    assert_eq!(plan.records[0].name, "app.example.test");
    assert_eq!(plan.records[0].content, "203.0.113.10");
    assert_eq!(plan.records[0].proxied, Some(false));
    assert_eq!(plan.records[2].record_type, CloudflareDnsRecordType::Aaaa);
    assert_eq!(plan.records[2].content, "2001:db8::a");
    assert_eq!(plan.records[2].proxied, Some(false));
    assert!(plan.warnings.is_empty());
}

#[test]
fn cloudflare_plan_warns_when_cloudflare_proxy_is_enabled_in_front_of_baia_proxy() {
    let request = DomainOnboardingRequest {
        hostname: "app.example.test".to_string(),
        ipv4_addresses: vec![Ipv4Addr::new(203, 0, 113, 10)],
        ipv6_addresses: Vec::new(),
        cloudflare_proxied: true,
        baia_proxy_enabled: true,
        acme_issuer: AcmeIssuer::LetsEncrypt,
        wildcard_certificate: false,
    };

    let plan = build_cloudflare_dns_plan(request).expect("DNS plan must be valid");

    assert_eq!(plan.records[0].proxied, Some(true));
    assert_eq!(
        plan.warnings,
        vec![CloudflareDnsPlanWarning::DoubleProxyMayBreakApplication]
    );
}

#[test]
fn cloudflare_plan_adds_caa_for_non_lets_encrypt_acme_issuer() {
    let request = DomainOnboardingRequest {
        hostname: "example.test".to_string(),
        ipv4_addresses: vec![Ipv4Addr::new(203, 0, 113, 10)],
        ipv6_addresses: Vec::new(),
        cloudflare_proxied: false,
        baia_proxy_enabled: true,
        acme_issuer: AcmeIssuer::Custom {
            caa_domain: "pki.goog".to_string(),
        },
        wildcard_certificate: true,
    };

    let plan = build_cloudflare_dns_plan(request).expect("DNS plan must be valid");
    let caa_records = plan
        .records
        .iter()
        .filter(|record| record.record_type == CloudflareDnsRecordType::Caa)
        .collect::<Vec<_>>();

    assert_eq!(caa_records.len(), 2);
    assert_eq!(
        caa_records[0]
            .data
            .as_ref()
            .expect("CAA must include data")
            .tag,
        "issue"
    );
    assert_eq!(
        caa_records[0]
            .data
            .as_ref()
            .expect("CAA must include data")
            .value,
        "pki.goog"
    );
    assert_eq!(
        caa_records[1]
            .data
            .as_ref()
            .expect("CAA must include data")
            .tag,
        "issuewild"
    );
    assert_eq!(caa_records[1].proxied, None);
}

#[test]
fn cloudflare_plan_rejects_domain_without_origin_address() {
    let request = DomainOnboardingRequest {
        hostname: "app.example.test".to_string(),
        ipv4_addresses: Vec::new(),
        ipv6_addresses: Vec::new(),
        cloudflare_proxied: false,
        baia_proxy_enabled: true,
        acme_issuer: AcmeIssuer::LetsEncrypt,
        wildcard_certificate: false,
    };

    let error = build_cloudflare_dns_plan(request).expect_err("origin address must be required");

    assert_eq!(
        error.to_string(),
        "at least one origin A or AAAA address is required"
    );
}

#[test]
fn known_acme_ca_catalog_identifies_common_caa_domains_by_alias() {
    let known = known_acme_cas();

    assert!(
        known
            .iter()
            .any(|ca| ca.name == "Let's Encrypt" && ca.caa_domain == "letsencrypt.org")
    );
    assert!(
        known
            .iter()
            .any(|ca| ca.name == "Google Trust Services" && ca.caa_domain == "pki.goog")
    );
    assert!(
        known
            .iter()
            .any(|ca| ca.name == "Sectigo" && ca.caa_domain == "sectigo.com")
    );

    assert_eq!(
        identify_known_acme_ca("google public ca")
            .expect("Google Public CA must be recognized")
            .caa_domain,
        "pki.goog"
    );
    assert_eq!(
        identify_known_acme_ca("zerossl")
            .expect("ZeroSSL must be recognized")
            .caa_domain,
        "sectigo.com"
    );
}

#[test]
fn cloudflare_create_record_requests_match_cloudflare_dns_api_shape() {
    let request = DomainOnboardingRequest {
        hostname: "example.test".to_string(),
        ipv4_addresses: vec![Ipv4Addr::new(203, 0, 113, 10)],
        ipv6_addresses: Vec::new(),
        cloudflare_proxied: true,
        baia_proxy_enabled: true,
        acme_issuer: AcmeIssuer::Custom {
            caa_domain: "pki.goog".to_string(),
        },
        wildcard_certificate: false,
    };
    let plan = build_cloudflare_dns_plan(request).expect("DNS plan must be valid");

    let requests = build_cloudflare_record_create_requests("zone_123", &plan);

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/zones/zone_123/dns_records");
    assert_eq!(requests[0].body["type"], "A");
    assert_eq!(requests[0].body["proxied"], true);
    assert_eq!(requests[1].body["type"], "CAA");
    assert_eq!(requests[1].body["data"]["tag"], "issue");
    assert_eq!(requests[1].body["data"]["value"], "pki.goog");
    assert!(requests[1].body.get("proxied").is_none());
}
