import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { isIP } from 'node:net';

export type CaddyfileOptions = {
  adminHostname: string;
};

const directOriginBlockPage =
  '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Direct origin access is not allowed</title><style>body{margin:0;font-family:system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;background:#f6f7f9;color:#17202a;display:grid;min-height:100vh;place-items:center}.panel{max-width:720px;padding:40px;border-top:4px solid #d33;background:#fff;box-shadow:0 16px 48px rgba(15,23,42,.12)}h1{font-size:28px;margin:0 0 12px}p{font-size:16px;line-height:1.55;margin:0 0 10px}.code{font-size:13px;color:#5b6673;text-transform:uppercase;letter-spacing:.08em}</style></head><body><main class="panel"><div class="code">Baia WAF 403</div><h1>Direct origin access is not allowed</h1><p>This hostname is not registered in Baia WAF or the request reached the origin directly by IP.</p><p>Register the domain in Baia WAF and access it through its configured hostname.</p></main></body></html>';

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
  const directOriginResponse = caddyQuotedString(directOriginBlockPage);

  return [
    '{',
    '\tadmin localhost:2019',
    '\temail {$BAIA_ACME_EMAIL}',
    '\tservers {',
    '\t\ttrusted_proxies static private_ranges',
    '\t}',
    '}',
    '',
    `http://${adminHostname} {`,
    '\trespond /health "ok" 200',
    '\tredir https://{host}{uri} permanent',
    '}',
    '',
    ':80 {',
    '\trespond /health "ok" 200',
    '\theader Content-Type "text/html; charset=utf-8"',
    `\trespond ${directOriginResponse} 403`,
    '}',
    '',
    `${adminHostname} {`,
    ...tlsPolicy,
    '\trespond /health "ok" 200',
    '\thandle /api* {',
    '\t\treverse_proxy core:8080',
    '\t}',
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

function caddyQuotedString(value: string): string {
  return `"${value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;
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
