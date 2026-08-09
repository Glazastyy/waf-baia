import { afterEach, describe, expect, test } from 'bun:test';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runComposePrepare } from './compose-prepare';

let currentRoot: string | undefined;

afterEach(async () => {
  if (currentRoot) {
    await rm(currentRoot, { recursive: true, force: true });
    currentRoot = undefined;
  }
});

async function createProjectFixture(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'baia-compose-prepare-'));
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
      '  publicUrl: https://admin.example.test',
      '  adminHostname: admin.example.test',
      'modules:',
      '  cloudflare:',
      '    enabled: false'
    ].join('\n')
  );
  return root;
}

describe('compose prepare', () => {
  test('creates all local files required for compose validation', async () => {
    const root = await createProjectFixture();

    const result = await runComposePrepare({ root });

    const secrets = await readFile(join(root, 'config', 'secrets.env'), 'utf8');
    const caddyfile = await readFile(join(root, 'config', 'generated', 'Caddyfile'), 'utf8');

    expect(result.created.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    expect(result.preserved).toEqual([]);
    expect(result.generated).toEqual(['config/generated/Caddyfile']);
    expect(secrets).toContain('BAIA_INITIAL_ADMIN_PASSWORD=');
    expect(secrets).not.toContain('change-this-initial-admin-password');
    expect(caddyfile).toContain('admin.example.test');
  });

  test('adds a missing initial admin password without replacing existing runtime files', async () => {
    const root = await createProjectFixture();
    await writeFile(
      join(root, 'config', 'secrets.env'),
      ['POSTGRES_PASSWORD=already-set', 'BAIA_ACME_EMAIL=admin@example.test'].join('\n')
    );
    await writeFile(join(root, 'config', 'platform.yaml'), 'platform:\n  adminHostname: existing.example.test\n');

    const result = await runComposePrepare({ root });
    const secrets = await readFile(join(root, 'config', 'secrets.env'), 'utf8');
    const platform = await readFile(join(root, 'config', 'platform.yaml'), 'utf8');

    expect(result.created).toEqual([]);
    expect(result.preserved.sort()).toEqual(['config/platform.yaml', 'config/secrets.env']);
    expect(secrets).toContain('POSTGRES_PASSWORD=already-set');
    expect(secrets).toContain('BAIA_INITIAL_ADMIN_PASSWORD=');
    expect(platform).toBe('platform:\n  adminHostname: existing.example.test\n');
  });
});
