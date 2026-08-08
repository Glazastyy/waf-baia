import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const composePath = join(import.meta.dir, '..', 'deploy/compose/docker-compose.yml');
const webDockerfilePath = join(import.meta.dir, '..', 'apps/web/Dockerfile');

describe('compose image tags', () => {
  test('uses verified service image tags', () => {
    const compose = readFileSync(composePath, 'utf8');

    expect(compose).toContain('image: postgres:18.1-alpine');
    expect(compose).toContain('image: redis:8.4.5-alpine');
    expect(compose).toContain('image: crowdsecurity/crowdsec:v1.7.8');
    expect(compose).toContain('image: powerdns/pdns-auth-51:5.1.3');
    expect(compose).toContain('image: poweradmin/poweradmin:stable');
  });

  test('does not use mutable latest tags for runtime dependencies', () => {
    const compose = readFileSync(composePath, 'utf8');

    expect(compose).not.toContain(':latest');
    expect(compose).not.toContain('pdns-auth-master');
  });

  test('pins web runtime caddy to a patch version', () => {
    const dockerfile = readFileSync(webDockerfilePath, 'utf8');

    expect(dockerfile).toContain('FROM caddy:2.11.4-alpine');
    expect(dockerfile).not.toContain('FROM caddy:2.10-alpine');
  });
});
