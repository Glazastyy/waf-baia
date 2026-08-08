import { afterEach, describe, expect, test } from 'bun:test';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runComposeUp } from './compose-up';

let currentRoot: string | undefined;

afterEach(async () => {
  if (currentRoot) {
    await rm(currentRoot, { recursive: true, force: true });
    currentRoot = undefined;
  }
});

async function createComposeFixture(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), 'baia-compose-up-'));
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
    ['platform:', '  publicUrl: https://admin.example.test', '  adminHostname: admin.example.test'].join('\n')
  );
  return root;
}

describe('compose up helper', () => {
  test('runs docker compose in detached mode and prints access details last', async () => {
    const root = await createComposeFixture();
    const output: string[] = [];
    const commands: string[][] = [];

    await runComposeUp({
      root,
      writeLine: (line) => output.push(line),
      runCommand: async (command) => {
        commands.push(command);
        output.push('docker build output');
        return 0;
      }
    });

    expect(commands).toEqual([
      [
        'docker',
        'compose',
        '--env-file',
        'config/secrets.env',
        '-f',
        'deploy/compose/docker-compose.yml',
        'up',
        '--build',
        '--detach'
      ]
    ]);
    expect(output.at(-5)).toBe('Baia WAF access');
    expect(output.at(-4)).toBe('Admin URL: https://admin.example.test/login');
    expect(output.at(-3)).toBe('Admin user: admin');
    expect(output.at(-2)?.startsWith('Initial admin password: ')).toBe(true);
    expect(output.at(-1)).toBe('Caddy admin API: http://localhost:2019');
  });

  test('adds missing initial admin password to existing secrets without replacing values', async () => {
    const root = await createComposeFixture();
    await writeFile(
      join(root, 'config', 'secrets.env'),
      ['POSTGRES_PASSWORD=existing-postgres', 'BAIA_ACME_EMAIL=ops@example.test', ''].join('\n')
    );
    const output: string[] = [];

    await runComposeUp({
      root,
      writeLine: (line) => output.push(line),
      runCommand: async () => 0
    });

    const secrets = await readFile(join(root, 'config', 'secrets.env'), 'utf8');
    expect(secrets).toContain('POSTGRES_PASSWORD=existing-postgres');
    expect(secrets).toContain('BAIA_INITIAL_ADMIN_PASSWORD=');
    expect(output.at(-2)?.startsWith('Initial admin password: ')).toBe(true);
  });
});
