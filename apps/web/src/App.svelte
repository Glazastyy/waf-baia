<script lang="ts">
  import { onMount } from 'svelte';
  import { createAuthClient, type AuthSession } from './lib/auth';
  import { localize, resolveLocale, supportedLocales, type Locale, type MessageKey } from './lib/i18n';

  type ServiceStatus = 'healthy' | 'degraded' | 'disabled';

  type Service = {
    name: string;
    roleKey: MessageKey;
    status: ServiceStatus;
    detailKey: MessageKey;
  };

  type Rule = {
    priority: number;
    nameKey: MessageKey;
    scopeKey: MessageKey;
    actionKey: MessageKey;
    enabled: boolean;
  };

  type Certificate = {
    domain: string;
    issuer: string;
    statusKey: MessageKey;
    renewalKey: MessageKey;
  };

  type PlannedDnsRecord = {
    type: string;
    name: string;
    value: string;
    modeKey: MessageKey;
  };

  type KnownCa = {
    name: string;
    caaDomain: string;
  };

  type ApplyMode = 'hotReload' | 'restartRequired' | 'externalApi' | 'noRuntimeApply';

  type ManagedComponent = {
    name: string;
    scope: string;
    applyMode: ApplyMode;
    coreManaged: boolean;
    capabilities: string[];
  };

  const languageStorageKey = 'baia.locale';
  const initialLocale = resolveLocale(localStorage.getItem(languageStorageKey) ?? navigator.language);
  persistLocale(initialLocale);
  const auth = createAuthClient();

  let locale = $state<Locale>(initialLocale);
  let i18n = $derived(localize(locale));
  let authSession = $state<AuthSession>({ authenticated: false, user: null, csrfToken: null });
  let authLoading = $state(true);
  let authSubmitting = $state(false);
  let loginUsername = $state('admin');
  let loginPassword = $state('');
  let loginError = $state('');
  let currentPassword = $state('');
  let newPassword = $state('');
  let changePasswordError = $state('');

  onMount(() => {
    void refreshSession();
  });

  let services = $state<Service[]>([
    { name: 'Core API', roleKey: 'service.core.role', status: 'healthy', detailKey: 'service.core.detail' },
    { name: 'Caddy', roleKey: 'service.caddy.role', status: 'healthy', detailKey: 'service.caddy.detail' },
    { name: 'PostgreSQL', roleKey: 'service.postgres.role', status: 'healthy', detailKey: 'service.postgres.detail' },
    { name: 'Redis', roleKey: 'service.redis.role', status: 'healthy', detailKey: 'service.redis.detail' },
    { name: 'CrowdSec', roleKey: 'service.crowdsec.role', status: 'disabled', detailKey: 'service.crowdsec.detail' },
    { name: 'PowerDNS', roleKey: 'service.powerdns.role', status: 'degraded', detailKey: 'service.powerdns.detail' }
  ]);

  let rules = $state<Rule[]>([
    {
      priority: 10,
      nameKey: 'rule.blockScanners.name',
      scopeKey: 'rule.blockScanners.scope',
      actionKey: 'action.block',
      enabled: true
    },
    {
      priority: 20,
      nameKey: 'rule.apiThrottle.name',
      scopeKey: 'rule.apiThrottle.scope',
      actionKey: 'action.rateLimit',
      enabled: true
    },
    {
      priority: 30,
      nameKey: 'rule.riskChallenge.name',
      scopeKey: 'rule.riskChallenge.scope',
      actionKey: 'action.challenge',
      enabled: false
    }
  ]);

  let certificates = $state<Certificate[]>([
    { domain: 'admin.waf.localhost', issuer: 'ACME HTTP-01', statusKey: 'certificate.admin.status', renewalKey: 'certificate.admin.renewal' },
    {
      domain: '*.example.test',
      issuer: 'ACME DNS-01',
      statusKey: 'certificate.wildcard.status',
      renewalKey: 'certificate.wildcard.renewal'
    }
  ]);

  let plannedDnsRecords = $state<PlannedDnsRecord[]>([
    { type: 'A', name: 'app.example.test', value: '203.0.113.10', modeKey: 'cloudflare.proxyOff' },
    { type: 'AAAA', name: 'app.example.test', value: '2001:db8::a', modeKey: 'cloudflare.proxyOff' },
    { type: 'CAA', name: 'example.test', value: '0 issue "pki.goog"', modeKey: 'cloudflare.proxyOff' }
  ]);

  let knownCas = $state<KnownCa[]>([
    { name: 'Let’s Encrypt', caaDomain: 'letsencrypt.org' },
    { name: 'Google Trust Services', caaDomain: 'pki.goog' },
    { name: 'Sectigo / ZeroSSL', caaDomain: 'sectigo.com' },
    { name: 'DigiCert', caaDomain: 'digicert.com' },
    { name: 'GlobalSign', caaDomain: 'globalsign.com' },
    { name: 'SSL.com', caaDomain: 'ssl.com' },
    { name: 'Buypass', caaDomain: 'buypass.com' }
  ]);

  let managedComponents = $state<ManagedComponent[]>([
    { name: 'Core', scope: 'Configuration, RBAC, audit, jobs', applyMode: 'hotReload', coreManaged: true, capabilities: ['API', 'RBAC', 'Audit'] },
    { name: 'Caddy', scope: 'Reverse proxy, TLS, WAF', applyMode: 'hotReload', coreManaged: true, capabilities: ['Admin API', 'Routes', 'TLS'] },
    { name: 'PowerDNS', scope: 'Authoritative DNS', applyMode: 'externalApi', coreManaged: true, capabilities: ['Zones', 'Records', 'DNSSEC'] },
    { name: 'Cloudflare', scope: 'External DNS and proxy', applyMode: 'externalApi', coreManaged: false, capabilities: ['A/AAAA', 'CAA', 'Proxy'] },
    { name: 'CrowdSec', scope: 'Decisions and remediation', applyMode: 'externalApi', coreManaged: true, capabilities: ['Decisions', 'Bouncers', 'AppSec'] },
    { name: 'PostgreSQL', scope: 'Persistent state', applyMode: 'restartRequired', coreManaged: true, capabilities: ['Migrations', 'Audit storage'] },
    { name: 'Redis', scope: 'Sessions, locks, cache', applyMode: 'restartRequired', coreManaged: true, capabilities: ['Locks', 'Sessions', 'Rate state'] },
    { name: 'ACME', scope: 'Certificates and renewals', applyMode: 'hotReload', coreManaged: true, capabilities: ['HTTP-01', 'DNS-01', 'CAA'] }
  ]);

  function statusClass(status: ServiceStatus): string {
    if (status === 'healthy') {
      return 'text-bg-success';
    }

    if (status === 'degraded') {
      return 'text-bg-warning';
    }

    return 'text-bg-secondary';
  }

  function toggleRule(rule: Rule): void {
    rule.enabled = !rule.enabled;
  }

  function changeLocale(value: string): void {
    const nextLocale = resolveLocale(value);
    locale = nextLocale;
    persistLocale(nextLocale);
  }

  function applyModeLabel(mode: ApplyMode): string {
    if (mode === 'hotReload') {
      return i18n.text('components.hotReload');
    }

    if (mode === 'restartRequired') {
      return i18n.text('components.restartRequired');
    }

    if (mode === 'externalApi') {
      return i18n.text('components.externalApi');
    }

    return i18n.text('components.noRuntimeApply');
  }

  function persistLocale(nextLocale: Locale): void {
    document.documentElement.lang = nextLocale;
    localStorage.setItem(languageStorageKey, nextLocale);
  }

  async function refreshSession(): Promise<void> {
    authLoading = true;
    try {
      authSession = await auth.session();
    } catch {
      authSession = { authenticated: false, user: null, csrfToken: null };
    } finally {
      authLoading = false;
    }
  }

  async function submitLogin(): Promise<void> {
    authSubmitting = true;
    loginError = '';
    try {
      authSession = await auth.login(loginUsername, loginPassword);
      loginPassword = '';
    } catch {
      loginError = i18n.text('auth.loginError');
    } finally {
      authSubmitting = false;
    }
  }

  async function submitLogout(): Promise<void> {
    authSession = await auth.logout();
  }

  async function submitPasswordChange(): Promise<void> {
    authSubmitting = true;
    changePasswordError = '';
    try {
      await auth.changePassword(currentPassword, newPassword);
      currentPassword = '';
      newPassword = '';
      authSession = await auth.session();
    } catch {
      changePasswordError = i18n.text('auth.changePasswordError');
    } finally {
      authSubmitting = false;
    }
  }
</script>

<main class="min-vh-100 bg-body-tertiary">
  {#if authLoading}
    <section class="auth-shell">
      <div class="auth-panel">
        <div class="d-flex align-items-center gap-2">
          <i class="bi bi-shield-lock fs-4"></i>
          <span class="fw-semibold">Baia WAF</span>
        </div>
        <div class="spinner-border mt-4" role="status" aria-label={i18n.text('auth.loading')}></div>
      </div>
    </section>
  {:else if !authSession.authenticated}
    <section class="auth-shell">
      <form class="auth-panel" onsubmit={(event) => { event.preventDefault(); void submitLogin(); }}>
        <div class="d-flex align-items-center gap-2 mb-4">
          <i class="bi bi-shield-lock fs-4"></i>
          <div>
            <div class="fw-semibold">Baia WAF</div>
            <div class="text-body-secondary small">{i18n.text('auth.signInSubtitle')}</div>
          </div>
        </div>
        {#if loginError}
          <div class="alert alert-danger" role="alert">{loginError}</div>
        {/if}
        <div class="mb-3">
          <label class="form-label" for="login-username">{i18n.text('auth.username')}</label>
          <input id="login-username" class="form-control" autocomplete="username" bind:value={loginUsername} required />
        </div>
        <div class="mb-3">
          <label class="form-label" for="login-password">{i18n.text('auth.password')}</label>
          <input id="login-password" class="form-control" type="password" autocomplete="current-password" bind:value={loginPassword} required />
        </div>
        <button class="btn btn-primary w-100" type="submit" disabled={authSubmitting}>
          {authSubmitting ? i18n.text('auth.signingIn') : i18n.text('auth.signIn')}
        </button>
      </form>
    </section>
  {:else if authSession.user?.passwordChangeRequired}
    <section class="auth-shell">
      <form class="auth-panel" onsubmit={(event) => { event.preventDefault(); void submitPasswordChange(); }}>
        <div class="d-flex align-items-center gap-2 mb-4">
          <i class="bi bi-key fs-4"></i>
          <div>
            <div class="fw-semibold">{i18n.text('auth.changePasswordTitle')}</div>
            <div class="text-body-secondary small">{i18n.text('auth.changePasswordSubtitle')}</div>
          </div>
        </div>
        {#if changePasswordError}
          <div class="alert alert-danger" role="alert">{changePasswordError}</div>
        {/if}
        <div class="mb-3">
          <label class="form-label" for="current-password">{i18n.text('auth.currentPassword')}</label>
          <input id="current-password" class="form-control" type="password" autocomplete="current-password" bind:value={currentPassword} required />
        </div>
        <div class="mb-3">
          <label class="form-label" for="new-password">{i18n.text('auth.newPassword')}</label>
          <input id="new-password" class="form-control" type="password" autocomplete="new-password" minlength="16" bind:value={newPassword} required />
          <div class="form-text">{i18n.text('auth.passwordPolicy')}</div>
        </div>
        <button class="btn btn-primary w-100" type="submit" disabled={authSubmitting}>
          {authSubmitting ? i18n.text('auth.saving') : i18n.text('auth.savePassword')}
        </button>
      </form>
    </section>
  {:else}
  <nav class="navbar navbar-expand-lg bg-dark border-bottom border-secondary" data-bs-theme="dark">
    <div class="container-fluid">
      <a class="navbar-brand fw-semibold" href="/">
        <i class="bi bi-shield-lock me-2"></i>
        Baia WAF
      </a>
      <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#admin-nav" aria-controls="admin-nav" aria-expanded="false" aria-label="Toggle navigation">
        <span class="navbar-toggler-icon"></span>
      </button>
      <div class="collapse navbar-collapse" id="admin-nav">
        <ul class="navbar-nav me-auto mb-2 mb-lg-0">
          <li class="nav-item"><a class="nav-link active" aria-current="page" href="/">{i18n.text('nav.overview')}</a></li>
          <li class="nav-item"><a class="nav-link" href="/applications">{i18n.text('nav.applications')}</a></li>
          <li class="nav-item"><a class="nav-link" href="/rules">{i18n.text('nav.rules')}</a></li>
          <li class="nav-item"><a class="nav-link" href="/dns">DNS</a></li>
          <li class="nav-item"><a class="nav-link" href="/audit">{i18n.text('nav.audit')}</a></li>
        </ul>
        <div class="d-flex flex-column flex-lg-row gap-2">
          <label class="visually-hidden" for="locale-select">{i18n.text('nav.language')}</label>
          <select id="locale-select" class="form-select form-select-sm language-select" value={locale} aria-label={i18n.text('nav.language')} onchange={(event) => changeLocale(event.currentTarget.value)}>
            {#each supportedLocales as supportedLocale (supportedLocale.code)}
              <option value={supportedLocale.code}>{supportedLocale.label}</option>
            {/each}
          </select>
          <button class="btn btn-outline-light btn-sm" type="button" title={i18n.text('nav.applyCaddy')}>
            <i class="bi bi-arrow-repeat"></i>
          </button>
          <button class="btn btn-warning btn-sm" type="button">
            <i class="bi bi-exclamation-triangle me-1"></i>
            {i18n.text('nav.pendingActions', { count: 2 })}
          </button>
          <button class="btn btn-outline-light btn-sm" type="button" onclick={() => void submitLogout()}>
            <i class="bi bi-box-arrow-right me-1"></i>
            {i18n.text('auth.logout')}
          </button>
        </div>
      </div>
    </div>
  </nav>

  <div class="container-fluid py-4">
    <div class="row g-3 mb-4">
      <div class="col-12 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="d-flex align-items-center justify-content-between">
              <h1 class="h5 mb-0">{i18n.text('summary.title')}</h1>
              <span class="badge text-bg-success">{i18n.text('summary.status')}</span>
            </div>
            <p class="text-body-secondary mb-0 mt-2">{i18n.text('summary.description', { services: 6, rules: 3, certificates: 2 })}</p>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">{i18n.text('metrics.protectedHosts')}</div>
            <div class="display-6">12</div>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">{i18n.text('metrics.blockedRequests')}</div>
            <div class="display-6">1,284</div>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">{i18n.text('metrics.p95Latency')}</div>
            <div class="display-6">82 ms</div>
          </div>
        </div>
      </div>
    </div>

    <div class="row g-4">
      <section class="col-12 col-xxl-7">
        <div class="card shadow-sm">
          <div class="card-header d-flex align-items-center justify-content-between">
            <span class="fw-semibold">{i18n.text('services.title')}</span>
            <button class="btn btn-outline-secondary btn-sm" type="button" title={i18n.text('services.refresh')}>
              <i class="bi bi-arrow-clockwise"></i>
            </button>
          </div>
          <div class="table-responsive">
            <table class="table table-hover align-middle mb-0">
              <thead>
                <tr>
                  <th scope="col">{i18n.text('table.service')}</th>
                  <th scope="col">{i18n.text('table.responsibility')}</th>
                  <th scope="col">{i18n.text('table.status')}</th>
                  <th scope="col">{i18n.text('table.detail')}</th>
                </tr>
              </thead>
              <tbody>
                {#each services as service (service.name)}
                  <tr>
                    <th scope="row">{service.name}</th>
                    <td>{i18n.text(service.roleKey)}</td>
                    <td><span class={`badge ${statusClass(service.status)}`}>{i18n.text(`status.${service.status}`)}</span></td>
                    <td>{i18n.text(service.detailKey)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <section class="col-12 col-xxl-5">
        <div class="card shadow-sm">
          <div class="card-header d-flex align-items-center justify-content-between">
            <span class="fw-semibold">{i18n.text('rules.title')}</span>
            <button class="btn btn-primary btn-sm" type="button">
              <i class="bi bi-plus-lg me-1"></i>
              {i18n.text('rules.new')}
            </button>
          </div>
          <div class="list-group list-group-flush">
            {#each rules as rule (rule.nameKey)}
              <div class="list-group-item">
                <div class="d-flex justify-content-between gap-3">
                  <div>
                    <div class="fw-semibold">{rule.priority}. {i18n.text(rule.nameKey)}</div>
                    <div class="text-body-secondary small">{i18n.text(rule.scopeKey)}</div>
                  </div>
                  <div class="d-flex align-items-center gap-2">
                    <span class="badge text-bg-info">{i18n.text(rule.actionKey)}</span>
                    <div class="form-check form-switch mb-0">
                      <input class="form-check-input" type="checkbox" checked={rule.enabled} aria-label={i18n.text('rules.toggle', { name: i18n.text(rule.nameKey) })} onchange={() => toggleRule(rule)} />
                    </div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      </section>

      <section class="col-12">
        <div class="card shadow-sm">
          <div class="card-header d-flex align-items-center justify-content-between">
            <span class="fw-semibold">{i18n.text('cloudflare.title')}</span>
            <span class="badge text-bg-secondary">{i18n.text('cloudflare.proxyOff')}</span>
          </div>
          <div class="card-body">
            <div class="alert alert-warning d-flex gap-2" role="alert">
              <i class="bi bi-exclamation-triangle-fill flex-shrink-0"></i>
              <div>
                <div class="fw-semibold">{i18n.text('cloudflare.warningTitle')}</div>
                <div>{i18n.text('cloudflare.doubleProxyWarning')}</div>
              </div>
            </div>
            <div class="row g-4">
              <div class="col-12 col-xl-7">
                <div class="fw-semibold mb-2">{i18n.text('cloudflare.dnsRecords')}</div>
                <div class="table-responsive">
                  <table class="table table-sm align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">Type</th>
                        <th scope="col">{i18n.text('certificates.domain')}</th>
                        <th scope="col">Value</th>
                        <th scope="col">{i18n.text('cloudflare.mode')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each plannedDnsRecords as record (`${record.type}:${record.name}:${record.value}`)}
                        <tr>
                          <td><span class="badge text-bg-light border">{record.type}</span></td>
                          <td>{record.name}</td>
                          <td><code>{record.value}</code></td>
                          <td>{i18n.text(record.modeKey)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </div>
              <div class="col-12 col-xl-5">
                <div class="fw-semibold mb-2">{i18n.text('cloudflare.caaTitle')}</div>
                <p class="text-body-secondary small">{i18n.text('cloudflare.caaDescription')}</p>
                <div class="d-flex flex-wrap gap-2">
                  {#each knownCas as ca (ca.name)}
                    <span class="badge text-bg-light border">{ca.name}: {ca.caaDomain}</span>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section class="col-12">
        <div class="card shadow-sm">
          <div class="card-header d-flex align-items-center justify-content-between">
            <span class="fw-semibold">{i18n.text('components.title')}</span>
            <button class="btn btn-outline-secondary btn-sm" type="button" title={i18n.text('services.refresh')}>
              <i class="bi bi-arrow-clockwise"></i>
            </button>
          </div>
          <div class="card-body">
            <p class="text-body-secondary small mb-3">{i18n.text('components.description')}</p>
            <div class="row g-3">
              {#each managedComponents as component (component.name)}
                <div class="col-12 col-md-6 col-xl-3">
                  <div class="border rounded-2 h-100 p-3 bg-body">
                    <div class="d-flex justify-content-between gap-2">
                      <div>
                        <div class="fw-semibold">{component.name}</div>
                        <div class="text-body-secondary small">{component.scope}</div>
                      </div>
                      <span class={`badge align-self-start ${component.coreManaged ? 'text-bg-primary' : 'text-bg-secondary'}`}>
                        {component.coreManaged ? i18n.text('components.coreManaged') : i18n.text('components.externalManaged')}
                      </span>
                    </div>
                    <div class="small mt-3">
                      <span class="text-body-secondary">{i18n.text('components.applyMode')}:</span>
                      <span class="fw-semibold">{applyModeLabel(component.applyMode)}</span>
                    </div>
                    <div class="d-flex flex-wrap gap-1 mt-3">
                      {#each component.capabilities as capability (`${component.name}:${capability}`)}
                        <span class="badge text-bg-light border">{capability}</span>
                      {/each}
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        </div>
      </section>

      <section class="col-12">
        <div class="card shadow-sm">
          <div class="card-header fw-semibold">{i18n.text('certificates.title')}</div>
          <div class="table-responsive">
            <table class="table align-middle mb-0">
              <thead>
                <tr>
                  <th scope="col">{i18n.text('certificates.domain')}</th>
                  <th scope="col">{i18n.text('certificates.issuer')}</th>
                  <th scope="col">{i18n.text('certificates.status')}</th>
                  <th scope="col">{i18n.text('certificates.renewal')}</th>
                </tr>
              </thead>
              <tbody>
                {#each certificates as certificate (certificate.domain)}
                  <tr>
                    <th scope="row">{certificate.domain}</th>
                    <td>{certificate.issuer}</td>
                    <td>{i18n.text(certificate.statusKey)}</td>
                    <td>{i18n.text(certificate.renewalKey)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      </section>
    </div>
  </div>
  {/if}
</main>

<style>
  :global(body) {
    min-width: 320px;
  }

  .navbar-brand {
    letter-spacing: 0;
  }

  .display-6 {
    font-weight: 600;
  }

  .language-select {
    min-width: 8.5rem;
  }

  .auth-shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 1.5rem;
  }

  .auth-panel {
    width: min(100%, 28rem);
    background: var(--bs-body-bg);
    border: 1px solid var(--bs-border-color);
    border-radius: .5rem;
    padding: 2rem;
    box-shadow: 0 1rem 3rem rgba(15, 23, 42, .16);
  }
</style>
