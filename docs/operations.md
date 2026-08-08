# Operations

This guide covers common operator tasks once the local stack has been initialized.

## Start And Stop

Start:

```sh
bun run compose:up
```

Stop:

```sh
bun run compose:down
```

Validate generated Compose configuration:

```sh
bun run compose:config
```

## Apply Configuration

The intended flow is:

1. Change settings in the panel.
2. Core validates the requested change.
3. Core writes `config/platform.yaml`.
4. Core applies only affected components.
5. Core records audit events.

Until the HTTP server is fully wired, the underlying tested contract is implemented in the Core configuration store.

## Reload Configuration From File

When `config/platform.yaml` is edited manually:

1. Reload through the Core.
2. Validate the loaded YAML.
3. Reject invalid state without applying changes.
4. Refresh panel state from the accepted configuration.

The API contract for this flow is `POST /api/configuration/reload`.

## Rotate Secrets

Update `config/secrets.env`, then restart affected services.

Examples:

- `POSTGRES_PASSWORD`: PostgreSQL and Core connection string
- `REDIS_PASSWORD`: Redis and Core connection string
- `BAIA_POWERDNS_API_KEY`: PowerDNS and Core
- `BAIA_CROWDSEC_API_KEY`: CrowdSec and Core
- `BAIA_CLOUDFLARE_API_TOKEN`: Core and Caddy when Cloudflare DNS-01 is used

Do not place secret values in `config/platform.yaml`.

## Caddy Plugin Verification

The repository includes a static regression test that ensures required Caddy modules remain in `services/caddy/Dockerfile`:

```sh
cargo test --test caddy_image
```

To verify the actual image, run this from a shell with Docker socket access:

```sh
docker build -t baia-caddy:test services/caddy
```

## Health Checks

The Compose stack defines health checks for PostgreSQL and Redis. Caddy exposes `/health` on port `80` in the initial Caddyfile.

Useful local checks:

```sh
bun run compose:config
cargo test
bun test
```

## Backups

Back up these volumes before destructive maintenance:

- `postgres-data`
- `powerdns-db-data`
- `redis-data` when Redis persistence matters
- `caddy-data`
- `caddy-config`
- `crowdsec-data`
- `crowdsec-config`

Back up these local files:

- `config/platform.yaml`
- `config/secrets.env`

Protect backups as sensitive material because they may allow full platform takeover.

## Updates

Before updating base images, Caddy modules or Rust/Bun dependencies:

1. Review upstream changelogs.
2. Run the full test suite.
3. Validate Compose.
4. Build the Caddy image with Docker access.
5. Test certificate and DNS workflows in a safe environment.

## Logs

Caddy logs are mounted into `caddy-logs` and consumed by CrowdSec. Core and service logs should be inspected through Docker Compose during local operation:

```sh
docker compose --env-file config/secrets.env -f deploy/compose/docker-compose.yml logs
```
