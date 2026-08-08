# Baia WAF

Baia WAF is a self-hosted WAF and reverse proxy control plane built as a monorepo.

The current repository contains the first implementation milestone:

- Rust Core domain contracts, configuration validation, admin bootstrap and Caddy JSON generation.
- Svelte 5 administrative panel using real Bootstrap.
- PostgreSQL initial schema.
- Central platform configuration and JSON Schema.
- Docker Compose development stack with separated services.
- Custom Caddy build with selected security modules.

Start with [docs/architecture.md](docs/architecture.md).
