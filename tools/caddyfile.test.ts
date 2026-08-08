import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { isLocalCertificateHostname, renderCaddyfile } from './caddyfile';

const caddyfilePath = join(import.meta.dir, '..', 'services/caddy/Caddyfile');

describe('static Caddyfile bootstrap', () => {
  test('serves local HTTP and HTTPS without exposing Caddy admin on an open interface', () => {
    const caddyfile = readFileSync(caddyfilePath, 'utf8');

    expect(caddyfile).toContain('admin localhost:2019');
    expect(caddyfile).not.toContain('admin 0.0.0.0:2019');
    expect(caddyfile).toContain(':80 {');
    expect(caddyfile).toContain('admin.waf.localhost {');
    expect(caddyfile).toContain('tls internal');
    expect(caddyfile).toContain('respond /health "ok" 200');
    expect(caddyfile).toContain('handle /api* {');
    expect(caddyfile).toContain('reverse_proxy core:8080');
    expect(caddyfile).toContain('reverse_proxy web:80');
    expect(caddyfile).toContain('http://admin.waf.localhost {');
    expect(caddyfile).toContain('Direct origin access is not allowed');
    expect(caddyfile).toContain('This hostname is not registered in Baia WAF');
    expect(caddyfile).toContain('respond "');
    expect(caddyfile).toContain(' 403');
  });
});

describe('generated Caddyfile bootstrap', () => {
  test('uses Caddy internal certificates for local hostnames', () => {
    const caddyfile = renderCaddyfile({ adminHostname: 'admin.waf.localhost' });

    expect(caddyfile).toContain('admin.waf.localhost {');
    expect(caddyfile).toContain('http://admin.waf.localhost {');
    expect(caddyfile).toContain('redir https://{host}{uri} permanent');
    expect(caddyfile).toContain('Direct origin access is not allowed');
    expect(caddyfile).toContain('tls internal');
    expect(caddyfile).toContain('handle /api* {');
    expect(caddyfile).toContain('reverse_proxy core:8080');
    expect(caddyfile).toContain('reverse_proxy web:80');
  });

  test('lets Caddy attempt public ACME certificates for public hostnames', () => {
    const caddyfile = renderCaddyfile({ adminHostname: 'waf.example.com' });

    expect(caddyfile).toContain('waf.example.com {');
    expect(caddyfile).toContain('http://waf.example.com {');
    expect(caddyfile).not.toContain('tls internal');
    expect(caddyfile).toContain('reverse_proxy web:80');
  });

  test('classifies local and public certificate hostnames', () => {
    expect(isLocalCertificateHostname('localhost')).toBe(true);
    expect(isLocalCertificateHostname('admin.waf.localhost')).toBe(true);
    expect(isLocalCertificateHostname('admin.example.test')).toBe(true);
    expect(isLocalCertificateHostname('127.0.0.1')).toBe(true);
    expect(isLocalCertificateHostname('10.10.1.5')).toBe(true);
    expect(isLocalCertificateHostname('waf.glazastov.com')).toBe(false);
  });
});
