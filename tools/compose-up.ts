import { constants } from 'node:fs';
import { access, appendFile, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { randomBytes } from 'node:crypto';
import { createInterface } from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';
import { runSetup } from './setup';
import { writeGeneratedCaddyfile } from './caddyfile';

export type ComposeUpOptions = {
  root?: string;
  writeLine?: (line: string) => void;
  runCommand?: (command: string[], cwd: string) => Promise<number>;
  runCommandWithOutput?: (command: string[], cwd: string) => Promise<CommandOutput>;
  confirmUpdate?: (details: GitUpdateDetails) => Promise<boolean>;
  checkForUpdates?: boolean;
};

type AccessDetails = {
  adminUrl: string;
  adminUser: string;
  initialAdminPassword: string;
  caddyAdminApi: string;
};

type CommandOutput = {
  exitCode: number;
  stdout: string;
};

type GitUpdateDetails = {
  upstream: string;
  commitsBehind: number;
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
  const runCommandWithOutput = options.runCommandWithOutput ?? runBufferedCommand;
  const confirmUpdate = options.confirmUpdate ?? confirmGitUpdate;

  if (options.checkForUpdates ?? true) {
    await updateFromGitHubIfRequested(root, writeLine, runCommand, runCommandWithOutput, confirmUpdate);
  }

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
  writeLine(`Initial admin password (only valid before first password change): ${access.initialAdminPassword}`);
  writeLine(`Caddy admin API: ${access.caddyAdminApi}`);
}

async function updateFromGitHubIfRequested(
  root: string,
  writeLine: (line: string) => void,
  runCommand: (command: string[], cwd: string) => Promise<number>,
  runCommandWithOutput: (command: string[], cwd: string) => Promise<CommandOutput>,
  confirmUpdate: (details: GitUpdateDetails) => Promise<boolean>
): Promise<void> {
  const insideWorkTree = await runCommandWithOutput(['git', 'rev-parse', '--is-inside-work-tree'], root);

  if (insideWorkTree.exitCode !== 0 || insideWorkTree.stdout.trim() !== 'true') {
    return;
  }

  const fetchExitCode = await runCommand(['git', 'fetch', '--quiet'], root);

  if (fetchExitCode !== 0) {
    writeLine('could not check GitHub updates; continuing with local checkout');
    return;
  }

  const upstream = await runCommandWithOutput(
    ['git', 'rev-parse', '--abbrev-ref', '--symbolic-full-name', '@{upstream}'],
    root
  );

  if (upstream.exitCode !== 0 || upstream.stdout.trim().length === 0) {
    return;
  }

  const behind = await runCommandWithOutput(['git', 'rev-list', '--count', 'HEAD..@{upstream}'], root);

  if (behind.exitCode !== 0) {
    writeLine('could not compare GitHub updates; continuing with local checkout');
    return;
  }

  const commitsBehind = Number.parseInt(behind.stdout.trim(), 10);

  if (!Number.isSafeInteger(commitsBehind) || commitsBehind <= 0) {
    return;
  }

  const details = {
    upstream: upstream.stdout.trim(),
    commitsBehind
  };
  writeLine(`GitHub update available: ${details.commitsBehind} commit(s) behind ${details.upstream}`);

  if (!(await confirmUpdate(details))) {
    writeLine('skipped GitHub update');
    return;
  }

  const pullExitCode = await runCommand(['git', 'pull', '--ff-only'], root);

  if (pullExitCode !== 0) {
    throw new Error('git pull --ff-only failed');
  }

  writeLine('updated local checkout from GitHub');
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

async function runBufferedCommand(command: string[], cwd: string): Promise<CommandOutput> {
  const child = Bun.spawn(command, {
    cwd,
    stdout: 'pipe',
    stderr: 'pipe'
  });
  const stdout = await new Response(child.stdout).text();
  await new Response(child.stderr).text();

  return {
    exitCode: await child.exited,
    stdout
  };
}

async function confirmGitUpdate(details: GitUpdateDetails): Promise<boolean> {
  if (!process.stdin.isTTY || !process.stdout.isTTY) {
    return false;
  }

  const readline = createInterface({ input, output });

  try {
    const answer = await readline.question(
      `Update from ${details.upstream} before starting? (${details.commitsBehind} commit(s)) [y/N] `
    );
    return answer.trim().toLowerCase() === 'y' || answer.trim().toLowerCase() === 'yes';
  } finally {
    readline.close();
  }
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
