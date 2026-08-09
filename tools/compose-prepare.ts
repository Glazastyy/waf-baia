import { appendFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { runSetup } from './setup';
import { writeGeneratedCaddyfile } from './caddyfile';

export type ComposePrepareResult = {
  created: string[];
  preserved: string[];
  generated: string[];
};

export type ComposePrepareOptions = {
  root?: string;
};

export async function runComposePrepare(options: ComposePrepareOptions = {}): Promise<ComposePrepareResult> {
  const root = options.root ?? process.cwd();
  const setup = await runSetup({ root });
  await ensureInitialAdminPassword(root);
  const caddyfile = await writeGeneratedCaddyfile(root);

  return {
    created: setup.created,
    preserved: setup.preserved,
    generated: [caddyfile]
  };
}

async function ensureInitialAdminPassword(root: string): Promise<void> {
  const secretsPath = join(root, 'config', 'secrets.env');
  const secrets = await readFile(secretsPath, 'utf8');

  if (parseEnv(secrets).has('BAIA_INITIAL_ADMIN_PASSWORD')) {
    return;
  }

  const separator = secrets.endsWith('\n') ? '' : '\n';
  await appendFile(secretsPath, `${separator}BAIA_INITIAL_ADMIN_PASSWORD=${randomSecret()}\n`, {
    mode: 0o600
  });
}

function parseEnv(raw: string): Map<string, string> {
  const values = new Map<string, string>();

  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();

    if (trimmed.length === 0 || trimmed.startsWith('#')) {
      continue;
    }

    const separatorIndex = trimmed.indexOf('=');

    if (separatorIndex === -1) {
      continue;
    }

    values.set(trimmed.slice(0, separatorIndex), trimmed.slice(separatorIndex + 1));
  }

  return values;
}

function randomSecret(): string {
  return randomBytes(32).toString('base64url');
}

if (import.meta.main) {
  const result = await runComposePrepare();

  for (const file of result.created) {
    console.log(`created ${file}`);
  }

  for (const file of result.preserved) {
    console.log(`preserved ${file}`);
  }

  for (const file of result.generated) {
    console.log(`generated ${file}`);
  }
}
