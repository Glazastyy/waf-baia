import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dir, '..');
const ciWorkflowPath = join(root, '.github/workflows/ci.yml');
const securityWorkflowPath = join(root, '.github/workflows/security.yml');
const dependabotPath = join(root, '.github/dependabot.yml');
const packageJsonPath = join(root, 'package.json');

describe('github workflows', () => {
  test('ci workflow covers bun, rust, compose and docker validation with least privilege defaults', () => {
    const workflow = readFileSync(ciWorkflowPath, 'utf8');

    expect(workflow).toContain('permissions:\n  contents: read');
    expect(workflow).toContain('concurrency:');
    expect(workflow).toContain('pull_request:');
    expect(workflow).toContain('push:');
    expect(workflow).not.toContain('pull_request_target');
    expect(workflow.match(/runs-on: ubuntu-latest/g)?.length).toBeGreaterThanOrEqual(3);
    expect(workflow).toContain('uses: actions/checkout@v7');
    expect(workflow).toContain('persist-credentials: false');
    expect(workflow).toContain('uses: oven-sh/setup-bun@v2');
    expect(workflow).toContain('bun-version: 1.3.14');
    expect(workflow).toContain('bun install --frozen-lockfile');
    expect(workflow).toContain('bun test');
    expect(workflow).toContain('bun run --cwd apps/web check');
    expect(workflow).toContain('bun run --cwd apps/web build');
    expect(workflow).toContain('cargo fmt --check');
    expect(workflow).toContain('cargo clippy --all-targets --all-features -- -D warnings');
    expect(workflow).toContain('cargo test');
    expect(workflow).toContain('bun run compose:prepare');
    expect(workflow).toContain('bun run compose:config');
    expect(workflow).toContain('docker compose --env-file config/secrets.env -f deploy/compose/docker-compose.yml build');
    expect(workflow.match(/timeout-minutes:/g)?.length).toBeGreaterThanOrEqual(3);
  });

  test('security workflow runs dependency review and codeql without privileged pull request triggers', () => {
    const workflow = readFileSync(securityWorkflowPath, 'utf8');

    expect(workflow).toContain('permissions:\n  contents: read');
    expect(workflow).toContain('pull_request:');
    expect(workflow).toContain('schedule:');
    expect(workflow).not.toContain('pull_request_target');
    expect(workflow).toContain('runs-on: ubuntu-latest');
    expect(workflow).toContain('uses: actions/dependency-review-action@v5');
    expect(workflow).toContain('fail-on-severity: moderate');
    expect(workflow).toContain('uses: github/codeql-action/init@v4');
    expect(workflow).toContain('uses: github/codeql-action/analyze@v4');
    expect(workflow).toContain('security-events: write');
    expect(workflow).toContain('javascript-typescript');
    expect(workflow).toContain('rust');
  });

  test('dependabot keeps ci actions and language ecosystems updated', () => {
    const config = readFileSync(dependabotPath, 'utf8');

    expect(config).toContain('package-ecosystem: github-actions');
    expect(config).toContain('package-ecosystem: cargo');
    expect(config).toContain('package-ecosystem: npm');
    expect(config).toContain('directory: /apps/web');
    expect(config.match(/interval: weekly/g)?.length).toBeGreaterThanOrEqual(4);
  });

  test('compose prepare script generates runtime files without starting containers', () => {
    const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));

    expect(packageJson.scripts['compose:prepare']).toBe('bun run tools/compose-prepare.ts');
  });
});
