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
        "github.com/caddy-dns/cloudflare",
        "github.com/mholt/caddy-ratelimit",
        "github.com/mholt/caddy-l4",
        "github.com/caddyserver/transform-encoder",
        "github.com/hslatman/caddy-crowdsec-bouncer/crowdsec",
        "github.com/hslatman/caddy-crowdsec-bouncer/http",
        "github.com/hslatman/caddy-crowdsec-bouncer/appsec",
        "github.com/hslatman/caddy-crowdsec-bouncer/layer4",
        "github.com/pberkel/caddy-storage-redis",
    ] {
        assert!(
            dockerfile.contains(module),
            "Caddy image must include module {module}"
        );
    }
}
