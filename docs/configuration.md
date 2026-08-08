# Configuration

Baia WAF uses two local runtime files:

- `config/platform.yaml` for non-secret platform settings
- `config/secrets.env` for passwords, API keys and tokens

Both files are ignored by Git. Examples are versioned as `config/platform.example.yaml` and `config/secrets.env.example`.

## Core As Source Of Truth

The intended production behavior is bidirectional:

- Panel change -> Core validates -> Core writes `config/platform.yaml` -> Core applies affected component
- File change -> Core reloads -> Core validates -> panel reflects the loaded state

The Core must reject invalid configuration before writing or applying it. Secrets stay outside `platform.yaml`.

## Runtime Files

Generate local files:

```sh
bun run setup
```

The setup command preserves existing local files.

Generate local files interactively:

```sh
bun run configure
```

The configure wizard asks for public URL, admin hostname, ACME email, PowerDNS, Cloudflare, CrowdSec, wildcard certificates and DNS-01 provider. It writes `config/platform.yaml` and `config/secrets.env`; internal passwords and service API keys are generated with cryptographic randomness.

## Platform Section

`platform.publicUrl` is the public URL used for links, login flows and generated admin references.

`platform.adminHostname` is the hostname that should reach the administrative panel.

## Modules

Each module has an `enabled` flag:

- `acme`
- `crowdsec`
- `captcha`
- `redis`
- `email`
- `powerdns`
- `cloudflare`
- `metrics`
- `experimental`

Disabled modules must remain unavailable until explicitly enabled.

## Services

`services.postgres` and `services.redis` describe the internal network location used by the Core.

`services.caddyAdminUrl` points to the Caddy Admin API. In the default Compose stack it is `http://caddy:2019`.

Changing stateful service host or port normally requires a coordinated restart.

## PowerDNS

PowerDNS can run integrated or external:

```yaml
integrations:
  powerdns:
    mode: integrated
    apiUrl: http://powerdns:8081/api/v1
    apiKeyEnv: BAIA_POWERDNS_API_KEY
```

`apiKeyEnv` references a key in `config/secrets.env`.

The Core should manage zones, records and DNSSEC through the PowerDNS HTTP API.

## Cloudflare

Cloudflare is optional and used for external DNS, proxied records and DNS-01.

```yaml
modules:
  cloudflare:
    enabled: true
integrations:
  cloudflare:
    apiTokenEnv: BAIA_CLOUDFLARE_API_TOKEN
    automaticDns:
      enabled: true
      defaultProxied: false
      requireDoubleProxyAcknowledgement: true
```

Use a token with minimum required permissions for the zones being managed. For DNS automation, the token needs zone read and DNS edit permissions.

When Cloudflare proxy is enabled in front of Baia WAF, the panel must warn about double proxy risk. Incorrect double proxy setups can make applications unreachable or create loops.

## ACME

Default simple certificates can use HTTP-01:

```yaml
tls:
  acme:
    emailEnv: BAIA_ACME_EMAIL
    http01Enabled: true
    dnsProvider: powerdns
    wildcardEnabled: false
```

Wildcard certificates require DNS-01 through `powerdns` or `cloudflare`.

When an ACME CA other than Let's Encrypt is used, the Core should plan CAA records using the known CA catalog or a manually supplied CAA domain.

## Secrets

`config/secrets.env` contains values such as:

```env
POSTGRES_PASSWORD=
REDIS_PASSWORD=
BAIA_POWERDNS_API_KEY=
BAIA_CROWDSEC_API_KEY=
BAIA_CLOUDFLARE_API_TOKEN=
BAIA_ACME_EMAIL=
```

Do not commit this file. Rotate values if they were exposed.

## Validation

Validate the Compose configuration after changes:

```sh
bun run compose:config
```

Validate Rust configuration code:

```sh
cargo test --test platform_config
cargo test --test config_file_sync
```
