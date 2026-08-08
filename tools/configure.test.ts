import { afterEach, describe, expect, test } from 'bun:test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runConfigure } from './configure';

let currentRoot: string | undefined;

afterEach(async () => {
  if (currentRoot) {
    await rm(currentRoot, { recursive: true, force: true });
    currentRoot = undefined;
  }
});

async function createConfigRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'baia-configure-'));
  currentRoot = root;
  await mkdir(join(root, 'config'), { recursive: true });
  return root;
}

describe('configure wizard', () => {
  test('generates platform and secret files from prompted answers', async () => {
    const root = await createConfigRoot();
    const answers = new Map([
      ['platform.publicUrl', 'https://waf.example.com'],
      ['platform.adminHostname', 'admin.example.com'],
      ['acme.email', 'ops@example.com'],
      ['modules.powerdns.enabled', 'yes'],
      ['modules.cloudflare.enabled', 'yes'],
      ['integrations.cloudflare.apiToken', 'cf-token-value'],
      ['integrations.cloudflare.automaticDns.defaultProxied', 'no'],
      ['modules.crowdsec.enabled', 'yes'],
      ['tls.acme.wildcardEnabled', 'yes'],
      ['tls.acme.dnsProvider', 'cloudflare']
    ]);

    const result = await runConfigure({
      root,
      force: true,
      ask: async (question) => answers.get(question.key) ?? question.defaultValue
    });

    const platform = await readFile(join(root, 'config', 'platform.yaml'), 'utf8');
    const secrets = await readFile(join(root, 'config', 'secrets.env'), 'utf8');

    expect(result.written.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    expect(platform).toContain('publicUrl: https://waf.example.com');
    expect(platform).toContain('adminHostname: admin.example.com');
    expect(platform).toContain('cloudflare:\n    enabled: true');
    expect(platform).toContain('dnsProvider: cloudflare');
    expect(platform).toContain('wildcardEnabled: true');
    expect(platform).not.toContain('cf-token-value');
    expect(secrets).toContain('BAIA_ACME_EMAIL=ops@example.com');
    expect(secrets).toContain('BAIA_CLOUDFLARE_API_TOKEN=cf-token-value');
    expect(readEnvValue(secrets, 'POSTGRES_PASSWORD')).toHaveLength(43);
    expect(readEnvValue(secrets, 'REDIS_PASSWORD')).toHaveLength(43);
    expect(readEnvValue(secrets, 'BAIA_POWERDNS_API_KEY')).toHaveLength(43);
    expect(readEnvValue(secrets, 'BAIA_CROWDSEC_API_KEY')).toHaveLength(43);
  });

  test('does not overwrite existing files without confirmation', async () => {
    const root = await createConfigRoot();
    await writeFile(join(root, 'config', 'platform.yaml'), 'platform: existing\n');
    await writeFile(join(root, 'config', 'secrets.env'), 'POSTGRES_PASSWORD=existing\n');

    const result = await runConfigure({
      root,
      ask: async (question) => (question.key === 'overwriteExistingFiles' ? 'no' : question.defaultValue)
    });

    expect(result.written).toEqual([]);
    expect(result.preserved.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    await expect(readFile(join(root, 'config', 'platform.yaml'), 'utf8')).resolves.toBe('platform: existing\n');
    await expect(readFile(join(root, 'config', 'secrets.env'), 'utf8')).resolves.toBe('POSTGRES_PASSWORD=existing\n');
  });
});

function readEnvValue(source: string, key: string): string {
  const line = source.split('\n').find((entry) => entry.startsWith(`${key}=`));

  if (!line) {
    throw new Error(`missing ${key}`);
  }

  return line.slice(key.length + 1);
}
