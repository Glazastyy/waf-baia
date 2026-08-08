import { constants } from 'node:fs';
import { access, mkdir, writeFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { createInterface } from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';
import { join } from 'node:path';

export type ConfigureQuestion = {
  key: string;
  label: string;
  defaultValue: string;
};

export type ConfigureResult = {
  written: string[];
  preserved: string[];
};

export type ConfigureOptions = {
  root?: string;
  force?: boolean;
  ask?: (question: ConfigureQuestion) => Promise<string>;
};

type ConfigureAnswers = {
  publicUrl: string;
  adminHostname: string;
  acmeEmail: string;
  powerdnsEnabled: boolean;
  cloudflareEnabled: boolean;
  cloudflareToken: string;
  cloudflareDefaultProxied: boolean;
  crowdsecEnabled: boolean;
  wildcardEnabled: boolean;
  dnsProvider: 'powerdns' | 'cloudflare';
};

const defaultAnswers: ConfigureAnswers = {
  publicUrl: 'https://admin.waf.localhost',
  adminHostname: 'admin.waf.localhost',
  acmeEmail: 'admin@example.test',
  powerdnsEnabled: true,
  cloudflareEnabled: false,
  cloudflareToken: '',
  cloudflareDefaultProxied: false,
  crowdsecEnabled: true,
  wildcardEnabled: false,
  dnsProvider: 'powerdns'
};

export async function runConfigure(options: ConfigureOptions = {}): Promise<ConfigureResult> {
  const root = options.root ?? process.cwd();
  const configDirectory = join(root, 'config');
  const platformPath = join(configDirectory, 'platform.yaml');
  const secretsPath = join(configDirectory, 'secrets.env');
  const result: ConfigureResult = {
    written: [],
    preserved: []
  };

  await mkdir(configDirectory, { recursive: true });

  const existingFiles = await existingRuntimeFiles(platformPath, secretsPath);

  if (existingFiles.length > 0 && !options.force) {
    const overwrite = await askBoolean(options, {
      key: 'overwriteExistingFiles',
      label: `Existing runtime files found: ${existingFiles.join(', ')}. Overwrite them?`,
      defaultValue: 'no'
    });

    if (!overwrite) {
      result.preserved.push(...existingFiles);
      return result;
    }
  }

  const answers = await collectAnswers(options);
  await writeFile(platformPath, renderPlatformYaml(answers), { mode: 0o600 });
  await writeFile(secretsPath, renderSecretsEnv(answers), { mode: 0o600 });
  result.written.push('config/platform.yaml', 'config/secrets.env');

  return result;
}

async function collectAnswers(options: ConfigureOptions): Promise<ConfigureAnswers> {
  const publicUrl = await askRequired(options, {
    key: 'platform.publicUrl',
    label: 'Public admin URL',
    defaultValue: defaultAnswers.publicUrl
  });
  const adminHostname = await askRequired(options, {
    key: 'platform.adminHostname',
    label: 'Admin hostname',
    defaultValue: hostnameFromUrl(publicUrl) ?? defaultAnswers.adminHostname
  });
  const acmeEmail = await askRequired(options, {
    key: 'acme.email',
    label: 'ACME email',
    defaultValue: defaultAnswers.acmeEmail
  });
  const powerdnsEnabled = await askBoolean(options, {
    key: 'modules.powerdns.enabled',
    label: 'Enable integrated PowerDNS?',
    defaultValue: yesNo(defaultAnswers.powerdnsEnabled)
  });
  const cloudflareEnabled = await askBoolean(options, {
    key: 'modules.cloudflare.enabled',
    label: 'Enable Cloudflare integration?',
    defaultValue: yesNo(defaultAnswers.cloudflareEnabled)
  });
  const cloudflareToken = cloudflareEnabled
    ? await askText(options, {
        key: 'integrations.cloudflare.apiToken',
        label: 'Cloudflare API token',
        defaultValue: ''
      })
    : '';
  const cloudflareDefaultProxied = cloudflareEnabled
    ? await askBoolean(options, {
        key: 'integrations.cloudflare.automaticDns.defaultProxied',
        label: 'Create Cloudflare records proxied by default?',
        defaultValue: yesNo(defaultAnswers.cloudflareDefaultProxied)
      })
    : false;
  const crowdsecEnabled = await askBoolean(options, {
    key: 'modules.crowdsec.enabled',
    label: 'Enable CrowdSec?',
    defaultValue: yesNo(defaultAnswers.crowdsecEnabled)
  });
  const wildcardEnabled = await askBoolean(options, {
    key: 'tls.acme.wildcardEnabled',
    label: 'Enable wildcard certificates?',
    defaultValue: yesNo(defaultAnswers.wildcardEnabled)
  });
  const dnsProvider = wildcardEnabled
    ? await askDnsProvider(options, cloudflareEnabled, powerdnsEnabled)
    : cloudflareEnabled
      ? 'cloudflare'
      : 'powerdns';

  return {
    publicUrl,
    adminHostname,
    acmeEmail,
    powerdnsEnabled,
    cloudflareEnabled,
    cloudflareToken,
    cloudflareDefaultProxied,
    crowdsecEnabled,
    wildcardEnabled,
    dnsProvider
  };
}

async function askDnsProvider(
  options: ConfigureOptions,
  cloudflareEnabled: boolean,
  powerdnsEnabled: boolean
): Promise<'powerdns' | 'cloudflare'> {
  const defaultValue = cloudflareEnabled ? 'cloudflare' : 'powerdns';
  const answer = await askText(options, {
    key: 'tls.acme.dnsProvider',
    label: 'DNS provider for ACME DNS-01 (powerdns/cloudflare)',
    defaultValue
  });
  const normalized = answer.trim().toLowerCase();

  if (normalized === 'cloudflare' && cloudflareEnabled) {
    return 'cloudflare';
  }

  if (normalized === 'powerdns' && powerdnsEnabled) {
    return 'powerdns';
  }

  return defaultValue;
}

async function existingRuntimeFiles(platformPath: string, secretsPath: string): Promise<string[]> {
  const files: string[] = [];

  if (await exists(platformPath)) {
    files.push('config/platform.yaml');
  }

  if (await exists(secretsPath)) {
    files.push('config/secrets.env');
  }

  return files;
}

function renderPlatformYaml(answers: ConfigureAnswers): string {
  return [
    'platform:',
    `  publicUrl: ${answers.publicUrl}`,
    `  adminHostname: ${answers.adminHostname}`,
    'modules:',
    '  acme:',
    '    enabled: true',
    '  crowdsec:',
    `    enabled: ${answers.crowdsecEnabled}`,
    '  captcha:',
    '    enabled: false',
    '  redis:',
    '    enabled: true',
    '  email:',
    '    enabled: false',
    '  powerdns:',
    `    enabled: ${answers.powerdnsEnabled}`,
    '  cloudflare:',
    `    enabled: ${answers.cloudflareEnabled}`,
    '  metrics:',
    '    enabled: true',
    '  experimental:',
    '    enabled: false',
    'services:',
    '  postgres:',
    '    host: postgres',
    '    port: 5432',
    '  redis:',
    '    host: redis',
    '    port: 6379',
    '  caddyAdminUrl: http://caddy:2019',
    'integrations:',
    '  powerdns:',
    '    mode: integrated',
    '    apiUrl: http://powerdns:8081/api/v1',
    '    apiKeyEnv: BAIA_POWERDNS_API_KEY',
    '  cloudflare:',
    '    apiTokenEnv: BAIA_CLOUDFLARE_API_TOKEN',
    '    automaticDns:',
    '      enabled: true',
    `      defaultProxied: ${answers.cloudflareDefaultProxied}`,
    '      requireDoubleProxyAcknowledgement: true',
    '  crowdsec:',
    '    localApiUrl: http://crowdsec:8080',
    '    apiKeyEnv: BAIA_CROWDSEC_API_KEY',
    'tls:',
    '  acme:',
    '    emailEnv: BAIA_ACME_EMAIL',
    '    http01Enabled: true',
    `    dnsProvider: ${answers.dnsProvider}`,
    `    wildcardEnabled: ${answers.wildcardEnabled}`,
    ''
  ].join('\n');
}

function renderSecretsEnv(answers: ConfigureAnswers): string {
  return [
    `POSTGRES_PASSWORD=${randomSecret()}`,
    `REDIS_PASSWORD=${randomSecret()}`,
    `BAIA_POWERDNS_API_KEY=${randomSecret()}`,
    `BAIA_CROWDSEC_API_KEY=${randomSecret()}`,
    `BAIA_CLOUDFLARE_API_TOKEN=${answers.cloudflareToken}`,
    `BAIA_ACME_EMAIL=${answers.acmeEmail}`,
    ''
  ].join('\n');
}

async function askRequired(options: ConfigureOptions, question: ConfigureQuestion): Promise<string> {
  const value = await askText(options, question);

  if (value.trim().length === 0) {
    return question.defaultValue;
  }

  return value.trim();
}

async function askBoolean(options: ConfigureOptions, question: ConfigureQuestion): Promise<boolean> {
  const value = (await askText(options, question)).trim().toLowerCase();

  if (['yes', 'y', 'sim', 's', 'true', '1'].includes(value)) {
    return true;
  }

  if (['no', 'n', 'nao', 'não', 'false', '0'].includes(value)) {
    return false;
  }

  return ['yes', 'true'].includes(question.defaultValue.toLowerCase());
}

async function askText(options: ConfigureOptions, question: ConfigureQuestion): Promise<string> {
  if (options.ask) {
    return options.ask(question);
  }

  const readline = createInterface({ input, output });

  try {
    const answer = await readline.question(`${question.label} [${question.defaultValue}]: `);
    return answer.trim().length === 0 ? question.defaultValue : answer;
  } finally {
    readline.close();
  }
}

function hostnameFromUrl(value: string): string | undefined {
  try {
    return new URL(value).hostname;
  } catch {
    return undefined;
  }
}

function yesNo(value: boolean): string {
  return value ? 'yes' : 'no';
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
  const result = await runConfigure();

  for (const file of result.written) {
    console.log(`written ${file}`);
  }

  for (const file of result.preserved) {
    console.log(`preserved ${file}`);
  }
}
