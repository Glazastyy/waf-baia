import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const webCaddyfilePath = join(import.meta.dir, '..', 'apps/web/Caddyfile');
const webDockerfilePath = join(import.meta.dir, '..', 'apps/web/Dockerfile');

describe('web Caddyfile', () => {
  test('serves the Svelte app for direct route visits', () => {
    const caddyfile = readFileSync(webCaddyfilePath, 'utf8');
    const dockerfile = readFileSync(webDockerfilePath, 'utf8');

    expect(caddyfile).toContain('try_files {path} /index.html');
    expect(caddyfile).toContain('file_server');
    expect(dockerfile).toContain('COPY apps/web/Caddyfile /etc/caddy/Caddyfile');
  });
});
