# Baia WAF

Baia WAF is a self-hosted WAF and reverse proxy control plane built as a monorepo.

The current repository contains the first implementation milestone:

- Rust Core domain contracts, configuration validation, admin bootstrap and Caddy JSON generation.
- Svelte 5 administrative panel using real Bootstrap and built-in i18n.
- PostgreSQL initial schema.
- Central platform configuration and JSON Schema.
- Docker Compose development stack with separated services.
- Custom Caddy build with selected security modules.

## Documentation

- [Getting Started](docs/getting-started.md)
- [Configuration](docs/configuration.md)
- [Components](docs/components.md)
- [Operations](docs/operations.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Architecture](docs/architecture.md)

## Quick Start

Prepare local runtime files:

```sh
bun run setup
```

Review `config/platform.yaml` and `config/secrets.env` when you want to change public hostnames, enable Cloudflare, or set a real ACME email. The setup command preserves existing local files and generates strong local secrets for the bundled services.

Validate the Compose stack:

```sh
bun run compose:config
```

Start the stack:

```sh
bun run compose:up
```

For the complete operator flow, start with [Getting Started](docs/getting-started.md).
