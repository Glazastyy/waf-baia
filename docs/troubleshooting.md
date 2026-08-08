# Troubleshooting

This guide lists common problems and direct checks.

## `bun run setup` Did Not Create Files

The setup command preserves existing files. Check:

```sh
ls config/platform.yaml config/secrets.env
```

If a file already exists, setup reports it as preserved.

For guided regeneration, use:

```sh
bun run configure
```

The wizard asks before overwriting existing runtime files.

## Compose Cannot Find Environment Variables

Run setup first:

```sh
bun run setup
```

Then validate:

```sh
bun run compose:config
```

If Compose still fails, check that `config/secrets.env` contains required keys from `config/secrets.env.example`.

## Compose Reports `manifest unknown`

This usually means an image tag in `deploy/compose/docker-compose.yml` does not exist in the registry.

Current verified runtime tags:

- `postgres:18.1-alpine`
- `redis:8.4.5-alpine`
- `crowdsecurity/crowdsec:v1.7.8`
- `powerdns/pdns-auth-51:5.1.3`
- `poweradmin/poweradmin:stable`
- `caddy:2.11.4-alpine`

Check tag regressions with:

```sh
bun test tools/compose-images.test.ts
```

Validate a tag manually:

```sh
docker manifest inspect crowdsecurity/crowdsec:v1.7.8
```

## Port 80 Or 443 Is Already In Use

The default Compose stack publishes:

- `80`
- `443`

Stop the conflicting service or edit `deploy/compose/docker-compose.yml` for local development.

## Docker Permission Denied

If Docker reports permission denied for `/var/run/docker.sock`, the current user cannot access Docker.

Check Docker access:

```sh
docker ps
```

Fix user/group access according to the host operating system, then retry.

## Caddy Image Build Fails

First run the static module test:

```sh
cargo test --test caddy_image
```

Then run the real build with Docker access:

```sh
docker build -t baia-caddy:test services/caddy
```

Failures usually come from network access to Go modules, incompatible plugin versions or a changed upstream module path.

## Caddy Admin API Is Unreachable

Check `services.caddyAdminUrl` in `config/platform.yaml`. The default is:

```yaml
services:
  caddyAdminUrl: http://caddy:2019
```

Inside Compose, `caddy` is the service hostname. The default stack does not publish the Caddy Admin API to the host.

## Cloudflare DNS Automation Does Not Work

Check:

- `modules.cloudflare.enabled` is true
- `BAIA_CLOUDFLARE_API_TOKEN` exists in `config/secrets.env`
- the token has zone read and DNS edit permissions
- the domain belongs to a zone visible to the token
- the desired A or AAAA origin address is present

If Cloudflare proxy is enabled, confirm the double-proxy warning was acknowledged and that the origin does not point back into a loop.

## Wildcard Certificate Fails

Wildcard certificates require DNS-01.

Check:

```yaml
tls:
  acme:
    dnsProvider: powerdns
    wildcardEnabled: true
```

Use `cloudflare` instead of `powerdns` when the managed zone is in Cloudflare.

## CAA Blocks Certificate Issuance

If using a CA other than Let's Encrypt, make sure the correct CAA records exist.

Known examples:

- Let's Encrypt: `letsencrypt.org`
- Google Trust Services: `pki.goog`
- Sectigo and ZeroSSL: `sectigo.com`
- DigiCert: `digicert.com`
- GlobalSign: `globalsign.com`
- SSL.com: `ssl.com`
- Buypass: `buypass.com`

For wildcard certificates, check both `issue` and `issuewild`.

## PowerDNS API Fails

Check:

- `modules.powerdns.enabled` is true
- `integrations.powerdns.apiUrl` points to the correct API base
- `BAIA_POWERDNS_API_KEY` exists in `config/secrets.env`
- the integrated `powerdns-db` service is running when using integrated mode

## CrowdSec Decisions Do Not Affect Caddy

Check:

- CrowdSec service is running
- Caddy was built with CrowdSec modules
- Caddy can reach the CrowdSec Local API
- `BAIA_CROWDSEC_API_KEY` matches the registered bouncer key
- Caddy trusted proxies are configured correctly when behind Cloudflare or another proxy

## Panel And File Configuration Differ

Use the Core reload flow after manual file edits:

```text
POST /api/configuration/reload
```

Invalid YAML or invalid platform state must be rejected. Fix the file and reload again.

## Tests Fail After A Config Change

Run focused checks first:

```sh
cargo test --test platform_config
cargo test --test config_file_sync
bun run compose:config
```

Then run the full suite:

```sh
cargo test
bun test
bun run --cwd apps/web check
bun run --cwd apps/web build
```
