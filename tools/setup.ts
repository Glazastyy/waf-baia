import { constants } from 'node:fs';
import { access, copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';

export type SetupResult = {
  created: string[];
  preserved: string[];
};

export type SetupOptions = {
  root?: string;
};

const generatedSecretKeys = new Set([
  'POSTGRES_PASSWORD',
  'REDIS_PASSWORD',
  'BAIA_POWERDNS_API_KEY',
  'BAIA_CROWDSEC_API_KEY'
]);

export async function runSetup(options: SetupOptions = {}): Promise<SetupResult> {
  const root = options.root ?? process.cwd();
  const configDirectory = join(root, 'config');
  const result: SetupResult = {
    created: [],
    preserved: []
  };

  await mkdir(configDirectory, { recursive: true });
  await ensureSecretsEnv(root, result);
  await ensurePlatformYaml(root, result);

  return result;
}

async function ensureSecretsEnv(root: string, result: SetupResult): Promise<void> {
  const targetPath = join(root, 'config', 'secrets.env');
  const relativePath = 'config/secrets.env';

  if (await exists(targetPath)) {
    result.preserved.push(relativePath);
    return;
  }

  const example = await readFile(join(root, 'config', 'secrets.env.example'), 'utf8');
  const generated = example
    .split(/\r?\n/)
    .filter((line) => line.length > 0)
    .map((line) => renderSecretLine(line))
    .join('\n');

  await writeFile(targetPath, `${generated}\n`, { mode: 0o600 });
  result.created.push(relativePath);
}

async function ensurePlatformYaml(root: string, result: SetupResult): Promise<void> {
  const targetPath = join(root, 'config', 'platform.yaml');
  const relativePath = 'config/platform.yaml';

  if (await exists(targetPath)) {
    result.preserved.push(relativePath);
    return;
  }

  await copyFile(join(root, 'config', 'platform.example.yaml'), targetPath);
  result.created.push(relativePath);
}

function renderSecretLine(line: string): string {
  const separatorIndex = line.indexOf('=');

  if (separatorIndex === -1) {
    return line;
  }

  const key = line.slice(0, separatorIndex);

  if (!generatedSecretKeys.has(key)) {
    return line;
  }

  return `${key}=${randomSecret()}`;
}

function randomSecret(): string {
  return randomBytes(32).toString('base64url');
}

async function exists(path: string): Promise<boolean> {
  try {
    await access(path, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

if (import.meta.main) {
  const result = await runSetup();

  for (const file of result.created) {
    console.log(`created ${file}`);
  }

  for (const file of result.preserved) {
    console.log(`preserved ${file}`);
  }

  if (result.created.length === 0 && result.preserved.length === 0) {
    console.log('nothing changed');
  }
}
