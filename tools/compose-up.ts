import { constants } from 'node:fs';
import { access, appendFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { runSetup } from './setup';
import { writeGeneratedCaddyfile } from './caddyfile';

export type ComposeUpOptions = {
  root?: string;
  writeLine?: (line: string) => void;
  runCommand?: (command: string[], cwd: string) => Promise<number>;
};

type AccessDetails = {
  adminUrl: string;
  adminUser: string;
  initialAdminPassword: string;
  caddyAdminApi: string;
};

const composeCommand = [
  'docker',
  'compose',
  '--env-file',
  'config/secrets.env',
  '-f',
  'deploy/compose/docker-compose.yml',
  'up',
  '--build',
  '--detach'
];

export async function runComposeUp(options: ComposeUpOptions = {}): Promise<void> {
  const root = options.root ?? process.cwd();
  const writeLine = options.writeLine ?? ((line: string) => console.log(line));
  const runCommand = options.runCommand ?? runInheritedCommand;

  const setup = await runSetup({ root });

  for (const file of setup.created) {
    writeLine(`created ${file}`);
  }

  for (const file of setup.preserved) {
    writeLine(`preserved ${file}`);
  }

  await ensureInitialAdminPassword(root);
  const caddyfile = await writeGeneratedCaddyfile(root);
  writeLine(`generated ${caddyfile}`);

  const exitCode = await runCommand(composeCommand, root);

  if (exitCode !== 0) {
    throw new Error(`docker compose exited with code ${exitCode}`);
  }

  const access = await readAccessDetails(root);
  writeLine('Baia WAF access');
  writeLine(`Admin URL: ${access.adminUrl}`);
  writeLine(`Admin user: ${access.adminUser}`);
  writeLine(`Initial admin password: ${access.initialAdminPassword}`);
  writeLine(`Caddy admin API: ${access.caddyAdminApi}`);
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

async function readAccessDetails(root: string): Promise<AccessDetails> {
  const platform = await readFile(join(root, 'config', 'platform.yaml'), 'utf8');
  const secrets = parseEnv(await readFile(join(root, 'config', 'secrets.env'), 'utf8'));
  const publicUrl = readYamlValue(platform, ['platform', 'publicUrl']) ?? 'http://localhost';
  const initialAdminPassword = secrets.get('BAIA_INITIAL_ADMIN_PASSWORD');

  if (!initialAdminPassword) {
    throw new Error('BAIA_INITIAL_ADMIN_PASSWORD is missing from config/secrets.env');
  }

  return {
    adminUrl: new URL('/login', normalizedUrl(publicUrl)).toString(),
    adminUser: 'admin',
    initialAdminPassword,
    caddyAdminApi: 'internal only (http://caddy:2019)'
  };
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

function normalizedUrl(value: string): string {
  if (/^https?:\/\//i.test(value)) {
    return value;
  }

  return `https://${value}`;
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

function randomSecret(): string {
  return randomBytes(32).toString('base64url');
}

async function runInheritedCommand(command: string[], cwd: string): Promise<number> {
  const child = Bun.spawn(command, {
    cwd,
    stdout: 'inherit',
    stderr: 'inherit'
  });

  return await child.exited;
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
  try {
    if (!(await exists(join(process.cwd(), 'config', 'secrets.env.example')))) {
      throw new Error('config/secrets.env.example was not found');
    }

    await runComposeUp();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'compose up failed';
    console.error(message);
    process.exit(1);
  }
}
