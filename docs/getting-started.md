# Getting Started

This guide brings a fresh Baia WAF checkout to a locally runnable stack with the smallest amount of manual configuration.

## Requirements

- Docker with Compose support
- Bun
- Cargo and a Rust toolchain for local Core development
- Ports `80`, `443` and `2019` available when running the default Compose stack

## First Start

Create local runtime files:

```sh
bun run setup
```

This creates:

- `config/secrets.env`, ignored by Git, with generated local passwords and API keys
- `config/platform.yaml`, ignored by Git, copied from `config/platform.example.yaml`

For a guided first configuration, use:

```sh
bun run configure
```

The wizard asks for hostnames, ACME email, provider choices and optional Cloudflare token. Internal passwords and API keys are generated securely.

Validate the Compose model:

```sh
bun run compose:config
```

Start the stack:

```sh
bun run compose:up
```

This starts Compose in detached mode. After Docker finishes building and starting services, the command prints the admin URL, admin user and initial password as the final output lines.

Stop the stack:

```sh
bun run compose:down
```

## First Things To Review

Open `config/platform.yaml` and review:

- `platform.publicUrl`
- `platform.adminHostname`
- `modules.cloudflare.enabled`
- `tls.acme.emailEnv`
- `tls.acme.dnsProvider`
- `tls.acme.wildcardEnabled`

Open `config/secrets.env` and review:

- `BAIA_ACME_EMAIL`
- `BAIA_INITIAL_ADMIN_PASSWORD`
- `BAIA_CLOUDFLARE_API_TOKEN`, when Cloudflare is enabled
- generated local passwords and API keys

## Plug-And-Play Model

The intended operator model is:

1. Use `bun run setup` once to create default local files, or `bun run configure` for a guided setup.
2. Manage day-to-day settings through the Core and panel.
3. Let the Core validate and persist panel changes back to `config/platform.yaml`.
4. If `config/platform.yaml` is edited manually, reload it through the Core so the panel reflects the file.

## Validation Commands

Run the main project checks:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
bun test
bun run --cwd apps/web check
bun run --cwd apps/web build
bun run compose:config
```

## Current Implementation State

The repository currently contains the control-plane contracts, configuration model, Caddy configuration generation, Cloudflare DNS planning, component catalog, setup utility, initial panel and Compose stack. Some API routes are intentionally still contracts until the full HTTP server and persistence layer are wired.
