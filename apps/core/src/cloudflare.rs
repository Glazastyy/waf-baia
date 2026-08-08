use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainOnboardingRequest {
    pub hostname: String,
    pub ipv4_addresses: Vec<Ipv4Addr>,
    pub ipv6_addresses: Vec<Ipv6Addr>,
    pub cloudflare_proxied: bool,
    pub baia_proxy_enabled: bool,
    pub acme_issuer: AcmeIssuer,
    pub wildcard_certificate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcmeIssuer {
    LetsEncrypt,
    Custom { caa_domain: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareDnsPlan {
    pub records: Vec<CloudflareDnsRecord>,
    pub warnings: Vec<CloudflareDnsPlanWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareDnsRecord {
    pub record_type: CloudflareDnsRecordType,
    pub name: String,
    pub content: String,
    pub ttl: u32,
    pub proxied: Option<bool>,
    pub data: Option<CaaRecordData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudflareDnsRecordType {
    A,
    Aaaa,
    Caa,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaaRecordData {
    pub flags: u8,
    pub tag: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnownAcmeCa {
    pub name: &'static str,
    pub caa_domain: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudflareApiRequest {
    pub method: &'static str,
    pub path: String,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudflareDnsPlanWarning {
    DoubleProxyMayBreakApplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloudflareDnsPlanError {
    EmptyHostname,
    InvalidHostname,
    MissingOriginAddress,
    MissingCaaDomain,
    InvalidCaaDomain,
}

impl Display for CloudflareDnsPlanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudflareDnsPlanError::EmptyHostname => formatter.write_str("hostname is required"),
            CloudflareDnsPlanError::InvalidHostname => {
                formatter.write_str("hostname must be a DNS name without scheme, path or spaces")
            }
            CloudflareDnsPlanError::MissingOriginAddress => {
                formatter.write_str("at least one origin A or AAAA address is required")
            }
            CloudflareDnsPlanError::MissingCaaDomain => {
                formatter.write_str("CAA domain is required for custom ACME issuer")
            }
            CloudflareDnsPlanError::InvalidCaaDomain => {
                formatter.write_str("CAA domain must be a DNS name without scheme, path or spaces")
            }
        }
    }
}

impl std::error::Error for CloudflareDnsPlanError {}

pub fn build_cloudflare_dns_plan(
    request: DomainOnboardingRequest,
) -> Result<CloudflareDnsPlan, CloudflareDnsPlanError> {
    validate_hostname(&request.hostname)?;

    if request.ipv4_addresses.is_empty() && request.ipv6_addresses.is_empty() {
        return Err(CloudflareDnsPlanError::MissingOriginAddress);
    }

    let mut records = Vec::new();

    for address in &request.ipv4_addresses {
        records.push(CloudflareDnsRecord {
            record_type: CloudflareDnsRecordType::A,
            name: request.hostname.clone(),
            content: address.to_string(),
            ttl: 1,
            proxied: Some(request.cloudflare_proxied),
            data: None,
        });
    }

    for address in &request.ipv6_addresses {
        records.push(CloudflareDnsRecord {
            record_type: CloudflareDnsRecordType::Aaaa,
            name: request.hostname.clone(),
            content: address.to_string(),
            ttl: 1,
            proxied: Some(request.cloudflare_proxied),
            data: None,
        });
    }

    append_caa_records(&mut records, &request)?;

    let warnings = if request.cloudflare_proxied && request.baia_proxy_enabled {
        vec![CloudflareDnsPlanWarning::DoubleProxyMayBreakApplication]
    } else {
        Vec::new()
    };

    Ok(CloudflareDnsPlan { records, warnings })
}

pub fn known_acme_cas() -> &'static [KnownAcmeCa] {
    &[
        KnownAcmeCa {
            name: "Let's Encrypt",
            caa_domain: "letsencrypt.org",
            aliases: &["lets encrypt", "letsencrypt", "let's encrypt", "le"],
        },
        KnownAcmeCa {
            name: "Google Trust Services",
            caa_domain: "pki.goog",
            aliases: &[
                "google",
                "google trust services",
                "google public ca",
                "gts",
                "pki.goog",
            ],
        },
        KnownAcmeCa {
            name: "Sectigo",
            caa_domain: "sectigo.com",
            aliases: &["sectigo", "comodo", "usertrust", "trust-provider"],
        },
        KnownAcmeCa {
            name: "ZeroSSL",
            caa_domain: "sectigo.com",
            aliases: &["zerossl", "zero ssl"],
        },
        KnownAcmeCa {
            name: "DigiCert",
            caa_domain: "digicert.com",
            aliases: &["digicert", "rapidssl", "geotrust", "thawte"],
        },
        KnownAcmeCa {
            name: "GlobalSign",
            caa_domain: "globalsign.com",
            aliases: &["globalsign", "global sign"],
        },
        KnownAcmeCa {
            name: "SSL.com",
            caa_domain: "ssl.com",
            aliases: &["ssl.com", "sslcom"],
        },
        KnownAcmeCa {
            name: "Buypass",
            caa_domain: "buypass.com",
            aliases: &["buypass", "buypass go ssl"],
        },
    ]
}

pub fn identify_known_acme_ca(value: &str) -> Option<&'static KnownAcmeCa> {
    let normalized = normalize_ca_name(value);

    known_acme_cas().iter().find(|ca| {
        normalize_ca_name(ca.name) == normalized
            || normalize_ca_name(ca.caa_domain) == normalized
            || ca
                .aliases
                .iter()
                .any(|alias| normalize_ca_name(alias) == normalized)
    })
}

pub fn build_cloudflare_record_create_requests(
    zone_id: &str,
    plan: &CloudflareDnsPlan,
) -> Vec<CloudflareApiRequest> {
    plan.records
        .iter()
        .map(|record| CloudflareApiRequest {
            method: "POST",
            path: format!("/zones/{zone_id}/dns_records"),
            body: cloudflare_record_body(record),
        })
        .collect()
}

fn append_caa_records(
    records: &mut Vec<CloudflareDnsRecord>,
    request: &DomainOnboardingRequest,
) -> Result<(), CloudflareDnsPlanError> {
    let caa_domain = match &request.acme_issuer {
        AcmeIssuer::LetsEncrypt => return Ok(()),
        AcmeIssuer::Custom { caa_domain } if caa_domain.trim().is_empty() => {
            return Err(CloudflareDnsPlanError::MissingCaaDomain);
        }
        AcmeIssuer::Custom { caa_domain } => caa_domain,
    };

    validate_caa_domain(caa_domain)?;

    records.push(caa_record(&request.hostname, "issue", caa_domain));

    if request.wildcard_certificate {
        records.push(caa_record(&request.hostname, "issuewild", caa_domain));
    }

    Ok(())
}

fn caa_record(hostname: &str, tag: &str, value: &str) -> CloudflareDnsRecord {
    CloudflareDnsRecord {
        record_type: CloudflareDnsRecordType::Caa,
        name: hostname.to_string(),
        content: format!("0 {tag} \"{value}\""),
        ttl: 1,
        proxied: None,
        data: Some(CaaRecordData {
            flags: 0,
            tag: tag.to_string(),
            value: value.to_string(),
        }),
    }
}

fn cloudflare_record_body(record: &CloudflareDnsRecord) -> Value {
    let mut body = json!({
        "type": record.record_type.as_cloudflare_type(),
        "name": record.name,
        "content": record.content,
        "ttl": record.ttl
    });

    if let Some(proxied) = record.proxied {
        body["proxied"] = json!(proxied);
    }

    if let Some(data) = &record.data {
        body["data"] = json!({
            "flags": data.flags,
            "tag": data.tag,
            "value": data.value
        });
    }

    body
}

fn validate_hostname(hostname: &str) -> Result<(), CloudflareDnsPlanError> {
    if hostname.trim().is_empty() {
        return Err(CloudflareDnsPlanError::EmptyHostname);
    }

    if has_invalid_dns_name_shape(hostname) {
        return Err(CloudflareDnsPlanError::InvalidHostname);
    }

    Ok(())
}

fn validate_caa_domain(domain: &str) -> Result<(), CloudflareDnsPlanError> {
    if has_invalid_dns_name_shape(domain) {
        return Err(CloudflareDnsPlanError::InvalidCaaDomain);
    }

    Ok(())
}

fn has_invalid_dns_name_shape(value: &str) -> bool {
    value.contains("://")
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_whitespace)
        || value.starts_with('.')
        || value.ends_with('.')
        || value.split('.').any(str::is_empty)
}

fn normalize_ca_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '.')
        .collect()
}

impl CloudflareDnsRecordType {
    fn as_cloudflare_type(&self) -> &'static str {
        match self {
            CloudflareDnsRecordType::A => "A",
            CloudflareDnsRecordType::Aaaa => "AAAA",
            CloudflareDnsRecordType::Caa => "CAA",
        }
    }
}
