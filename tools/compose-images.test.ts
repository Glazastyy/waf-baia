import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const composePath = join(import.meta.dir, '..', 'deploy/compose/docker-compose.yml');
const webDockerfilePath = join(import.meta.dir, '..', 'apps/web/Dockerfile');
const coreServerPath = join(import.meta.dir, '..', 'apps/core/src/server.rs');
const postgresInitPath = join(import.meta.dir, '..', 'deploy/compose/postgres-init/00-create-powerdns.sql');

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

  test('only publishes public HTTP and HTTPS ports on the host', () => {
    const compose = readFileSync(composePath, 'utf8');

    expect(compose).toContain('- "80:80"');
    expect(compose).toContain('- "443:443"');
    expect(compose).not.toContain('- "2019:2019"');
    expect(compose).not.toContain('- "5432:5432"');
    expect(compose).not.toContain('- "6379:6379"');
    expect(compose).not.toContain('- "8081:8081"');
    expect(compose).not.toContain('- "53:53"');
  });

  test('uses one postgres instance with separate databases for core and powerdns', () => {
    const compose = readFileSync(composePath, 'utf8');
    const postgresInit = readFileSync(postgresInitPath, 'utf8');

    expect(compose).toContain('postgres:');
    expect(compose).not.toContain('powerdns-db:');
    expect(compose).toContain('postgres-init/00-create-powerdns.sql');
    expect(postgresInit).toContain('CREATE DATABASE powerdns OWNER baia');
    expect(compose).toContain('PDNS_AUTH_GPGSQL_HOST: postgres');
    expect(compose).toContain('DB_HOST: postgres');
    expect(compose).not.toContain('powerdns-db-data:');
  });

  test('persists core state with explicit postgres transactions', () => {
    const server = readFileSync(coreServerPath, 'utf8');

    expect(server).toContain('transaction().await?');
    expect(server).toContain('transaction.commit().await?');
  });
});
