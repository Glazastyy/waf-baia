import { afterEach, describe, expect, test } from 'bun:test';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runSetup } from './setup';

let currentRoot: string | undefined;

afterEach(async () => {
  if (currentRoot) {
    await rm(currentRoot, { recursive: true, force: true });
    currentRoot = undefined;
  }
});

async function createProjectFixture(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'baia-setup-'));
  currentRoot = root;
  await mkdir(join(root, 'config'), { recursive: true });
  await writeFile(
    join(root, 'config', 'secrets.env.example'),
    [
      'POSTGRES_PASSWORD=change-this-postgres-password',
      'REDIS_PASSWORD=change-this-redis-password',
      'BAIA_POWERDNS_API_KEY=change-this-powerdns-api-key',
      'BAIA_CROWDSEC_API_KEY=change-this-crowdsec-api-key',
      'BAIA_INITIAL_ADMIN_PASSWORD=change-this-initial-admin-password',
      'BAIA_CLOUDFLARE_API_TOKEN=',
      'BAIA_ACME_EMAIL=admin@example.test'
    ].join('\n')
  );
  await writeFile(
    join(root, 'config', 'platform.example.yaml'),
    [
      'platform:',
      '  publicUrl: https://admin.waf.localhost',
      '  adminHostname: admin.waf.localhost',
      'modules:',
      '  cloudflare:',
      '    enabled: false'
    ].join('\n')
  );
  return root;
}

describe('setup', () => {
  test('creates local runtime files without placeholder secrets', async () => {
    const root = await createProjectFixture();

    const result = await runSetup({ root });

    const secrets = await readFile(join(root, 'config', 'secrets.env'), 'utf8');
    const platform = await readFile(join(root, 'config', 'platform.yaml'), 'utf8');

    expect(result.created.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    expect(result.preserved).toEqual([]);
    expect(secrets).toContain('BAIA_CLOUDFLARE_API_TOKEN=');
    expect(secrets).toContain('BAIA_ACME_EMAIL=admin@example.test');
    expect(secrets).not.toContain('change-this-postgres-password');
    expect(secrets).not.toContain('change-this-redis-password');
    expect(secrets).not.toContain('change-this-powerdns-api-key');
    expect(secrets).not.toContain('change-this-crowdsec-api-key');
    expect(secrets).not.toContain('change-this-initial-admin-password');
    expect(platform).toContain('publicUrl: https://admin.waf.localhost');
  });

  test('preserves existing local runtime files', async () => {
    const root = await createProjectFixture();
    await writeFile(join(root, 'config', 'secrets.env'), 'POSTGRES_PASSWORD=already-set\n');
    await writeFile(join(root, 'config', 'platform.yaml'), 'platform:\n  publicUrl: https://existing.example\n');

    const result = await runSetup({ root });

    expect(result.created).toEqual([]);
    expect(result.preserved.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    await expect(readFile(join(root, 'config', 'secrets.env'), 'utf8')).resolves.toBe('POSTGRES_PASSWORD=already-set\n');
  });
});
