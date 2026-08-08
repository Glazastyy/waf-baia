# Components

Baia WAF is designed around a Core-managed component catalog. Each component declares settings, secrets, capabilities and how changes are applied.

## Apply Modes

`HotReload`: The Core can apply the change without restarting the container. Caddy config through the Admin API is the primary example.

`ExternalApi`: The Core sends the change to a service or provider API. PowerDNS, Cloudflare and CrowdSec fit here.

`RestartRequired`: The change affects runtime wiring and requires a coordinated restart. PostgreSQL and Redis host/port changes fit here.

`NoRuntimeApply`: The setting does not require a backend apply step.

## Core

Responsible for:

- Admin API
- configuration validation
- bidirectional config file sync
- RBAC and audit
- orchestration jobs
- Caddy JSON generation
- DNS and certificate planning

Expected management surface: Core and panel.

## Web Panel

Responsible for:

- i18n administrative UI
- operator workflows
- component status
- pending actions
- guided configuration

Expected management surface: browser UI backed by Core APIs.

## Caddy

Responsible for:

- TLS termination
- reverse proxy
- routing
- WAF enforcement
- rate limiting
- CrowdSec bouncer integration
- Layer4 protection

The custom Caddy image includes:

- `github.com/caddy-dns/cloudflare`
- `github.com/mholt/caddy-ratelimit`
- `github.com/mholt/caddy-l4`
- `github.com/caddyserver/transform-encoder`
- `github.com/hslatman/caddy-crowdsec-bouncer/crowdsec`
- `github.com/hslatman/caddy-crowdsec-bouncer/http`
- `github.com/hslatman/caddy-crowdsec-bouncer/appsec`
- `github.com/hslatman/caddy-crowdsec-bouncer/layer4`
- `github.com/pberkel/caddy-storage-redis@v1.8.1`

Expected apply mode: hot reload through Caddy Admin API.

## PostgreSQL

Responsible for persistent state:

- users
- roles
- protected applications
- upstreams
- WAF rules
- DNS zones and records
- certificates
- audit events
- security events

Expected apply mode: restart required for connection settings, migrations for schema changes.

## Redis

Responsible for temporary and distributed state:

- sessions
- locks
- cache
- queues
- rate limit state
- Caddy storage when configured

Expected apply mode: restart required for connection settings.

## PowerDNS

Responsible for authoritative DNS when integrated or when an external PowerDNS server is connected.

The Core should manage:

- zones
- records
- DNSSEC
- connectivity checks

Expected apply mode: external API.

## Cloudflare

Responsible for optional external DNS and proxy features.

The Core should manage:

- zone discovery
- A and AAAA records
- CAA records
- DNS-only or proxied mode
- ACME DNS-01 support

Expected apply mode: external API.

## CrowdSec

Responsible for security decisions and remediation feeds.

The Core should manage:

- Local API connectivity
- bouncers
- decisions
- allowlists and blocklists
- AppSec status

Expected apply mode: external API.

## ACME

Responsible for certificates and renewals through Caddy/CertMagic.

The Core should manage:

- issuer policy
- HTTP-01 and DNS-01 selection
- wildcard enablement
- CAA planning
- certificate state and renewal errors

Expected apply mode: hot reload through Caddy configuration.
