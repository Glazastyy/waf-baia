use std::fs;
use std::path::PathBuf;

#[test]
fn caddy_image_builds_with_required_waf_dns_and_storage_modules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve");
    let dockerfile = fs::read_to_string(root.join("services/caddy/Dockerfile"))
        .expect("Caddy Dockerfile must be readable");

    for module in [
        "github.com/caddy-dns/cloudflare@v0.2.4",
        "github.com/mholt/caddy-ratelimit@v0.1.0",
        "github.com/mholt/caddy-l4@v0.1.2",
        "github.com/caddyserver/transform-encoder@v0.0.0-20260423033309-ba4124974830",
        "github.com/hslatman/caddy-crowdsec-bouncer/crowdsec@v0.14.1",
        "github.com/hslatman/caddy-crowdsec-bouncer/http@v0.14.1",
        "github.com/hslatman/caddy-crowdsec-bouncer/appsec@v0.14.1",
        "github.com/hslatman/caddy-crowdsec-bouncer/layer4@v0.14.1",
        "github.com/pberkel/caddy-storage-redis@v1.8.1",
    ] {
        assert!(
            dockerfile.contains(module),
            "Caddy image must include module {module}"
        );
    }

    assert!(dockerfile.contains("ARG CADDY_VERSION=2.11.4"));
}
