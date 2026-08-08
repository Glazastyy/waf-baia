import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { isIP } from 'node:net';

export type CaddyfileOptions = {
  adminHostname: string;
};

export async function writeGeneratedCaddyfile(root: string): Promise<string> {
  const platform = await readFile(join(root, 'config', 'platform.yaml'), 'utf8');
  const adminHostname = readYamlValue(platform, ['platform', 'adminHostname']) ?? 'admin.waf.localhost';
  const target = join(root, 'config', 'generated', 'Caddyfile');
  await mkdir(dirname(target), { recursive: true });
  await writeFile(target, renderCaddyfile({ adminHostname }), { mode: 0o600 });
  return 'config/generated/Caddyfile';
}

export function renderCaddyfile(options: CaddyfileOptions): string {
  const adminHostname = normalizeHostname(options.adminHostname);
  const tlsPolicy = isLocalCertificateHostname(adminHostname) ? ['\ttls internal'] : [];

  return [
    '{',
    '\tadmin localhost:2019',
    '\temail {$BAIA_ACME_EMAIL}',
    '\tservers {',
    '\t\ttrusted_proxies static private_ranges',
    '\t}',
    '}',
    '',
    ':80 {',
    '\trespond /health "ok" 200',
    '}',
    '',
    `${adminHostname} {`,
    ...tlsPolicy,
    '\trespond /health "ok" 200',
    '\treverse_proxy web:80',
    '}',
    ''
  ].join('\n');
}

export function isLocalCertificateHostname(hostname: string): boolean {
  const normalized = normalizeHostname(hostname);
  const ipVersion = isIP(normalized);

  if (ipVersion !== 0) {
    return isLocalIpAddress(normalized);
  }

  if (normalized === 'localhost') {
    return true;
  }

  return ['.localhost', '.test', '.invalid', '.example'].some((suffix) => normalized.endsWith(suffix));
}

function isLocalIpAddress(value: string): boolean {
  if (value === '127.0.0.1' || value === '::1') {
    return true;
  }

  const parts = value.split('.').map((part) => Number.parseInt(part, 10));

  if (parts.length !== 4 || parts.some((part) => Number.isNaN(part))) {
    return false;
  }

  const [first, second] = parts;

  if (first === 10 || first === 127) {
    return true;
  }

  if (first === 172 && second >= 16 && second <= 31) {
    return true;
  }

  return first === 192 && second === 168;
}

function normalizeHostname(hostname: string): string {
  return hostname.trim().toLowerCase().replace(/\.$/, '');
}

function readYamlValue(raw: string, path: string[]): string | undefined {
  const lines = raw.split(/\r?\n/);
  let currentSection = '';

  for (const line of lines) {
    const sectionMatch = /^([A-Za-z0-9_-]+):\s*$/.exec(line);

    if (sectionMatch) {
      currentSection = sectionMatch[1] ?? '';
      continue;
    }

    const valueMatch = /^\s{2}([A-Za-z0-9_-]+):\s*(.+?)\s*$/.exec(line);

    if (valueMatch && currentSection === path[0] && valueMatch[1] === path[1]) {
      return unquote(valueMatch[2] ?? '');
    }
  }

  return undefined;
}

function unquote(value: string): string {
  const trimmed = value.trim();

  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    return trimmed.slice(1, -1);
  }

  return trimmed;
}
