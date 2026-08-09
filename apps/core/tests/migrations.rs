use std::fs;
use std::path::PathBuf;

#[test]
fn cloudflare_dns_plan_migration_references_existing_application_table() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve");
    let initial = fs::read_to_string(root.join("apps/core/migrations/0001_initial.sql"))
        .expect("initial migration must be readable");
    let cloudflare_dns =
        fs::read_to_string(root.join("apps/core/migrations/0002_cloudflare_dns_plans.sql"))
            .expect("cloudflare dns migration must be readable");

    assert!(initial.contains("CREATE TABLE applications"));
    assert!(cloudflare_dns.contains("REFERENCES applications(id)"));
    assert!(!cloudflare_dns.contains("protected_applications"));
}

#[test]
fn core_schema_migrations_are_explicitly_transactional() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve");
    let migrations = [
        root.join("apps/core/migrations/0001_initial.sql"),
        root.join("apps/core/migrations/0002_cloudflare_dns_plans.sql"),
    ];

    for migration in migrations {
        let raw = fs::read_to_string(&migration).expect("migration must be readable");
        let normalized = raw.trim();

        assert!(normalized.starts_with("BEGIN;"));
        assert!(normalized.ends_with("COMMIT;"));
    }
}
