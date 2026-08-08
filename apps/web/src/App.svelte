<script lang="ts">
  import { onMount } from 'svelte';
  import { createApplicationsClient } from './lib/applications';
  import { createAuditClient } from './lib/audit';
  import { createAuthClient, type AuthSession } from './lib/auth';
  import {
    createCertificatesClient,
    type CertificateChallengeType,
    type CertificateStatus
  } from './lib/certificates';
  import {
    componentStatusesFromConfiguration,
    createConfigClient,
    type ComponentConfigurationStatus,
    type ComponentStatus
  } from './lib/config';
  import { createEmptyDashboard, dashboardSummary, type DashboardState } from './lib/dashboard';
  import { createDnsClient, type DnsRecordType } from './lib/dns';
  import { localize, resolveLocale, supportedLocales, type Locale } from './lib/i18n';
  import { createRateLimitsClient, type RateLimitAction } from './lib/rate-limits';
  import { createWafRulesClient, type WafRuleAction } from './lib/waf-rules';
  import {
    adminNavigation,
    pathForRoute,
    resolveAdminRoute,
    shouldRedirectAuthenticatedUser,
    type AdminRoute
  } from './lib/routes';

  const languageStorageKey = 'baia.locale';
  const initialLocale = resolveLocale(localStorage.getItem(languageStorageKey) ?? navigator.language);
  persistLocale(initialLocale);
  const auth = createAuthClient();
  const configClient = createConfigClient();
  const applicationsClient = createApplicationsClient();
  const wafRulesClient = createWafRulesClient();
  const dnsClient = createDnsClient();
  const auditClient = createAuditClient();
  const certificatesClient = createCertificatesClient();
  const rateLimitsClient = createRateLimitsClient();

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
  let applicationName = $state('');
  let applicationHostname = $state('');
  let applicationUpstreamDial = $state('');
  let applicationsLoading = $state(false);
  let applicationsSubmitting = $state(false);
  let applicationsError = $state('');
  let wafRuleName = $state('');
  let wafRuleApplicationId = $state('');
  let wafRulePriority = $state(100);
  let wafRuleAction = $state<WafRuleAction>('block');
  let wafRulePathPrefix = $state('');
  let wafRulesLoading = $state(false);
  let wafRulesSubmitting = $state(false);
  let wafRulesError = $state('');
  let rateLimitName = $state('');
  let rateLimitApplicationId = $state('');
  let rateLimitPathPrefix = $state('');
  let rateLimitRequests = $state(100);
  let rateLimitWindowSeconds = $state(60);
  let rateLimitAction = $state<RateLimitAction>('block');
  let rateLimitsLoading = $state(false);
  let rateLimitsSubmitting = $state(false);
  let rateLimitsError = $state('');
  let dnsZoneName = $state('');
  let dnsRecordName = $state('');
  let dnsRecordType = $state<DnsRecordType>('A');
  let dnsRecordContent = $state('');
  let dnsRecordTtl = $state(300);
  let dnsRecordProxied = $state(false);
  let dnsLoading = $state(false);
  let dnsSubmitting = $state(false);
  let dnsError = $state('');
  let auditLoading = $state(false);
  let auditError = $state('');
  let certificateApplicationId = $state('');
  let certificateDomain = $state('');
  let certificateIssuer = $state('letsencrypt');
  let certificateChallengeType = $state<CertificateChallengeType>('http_01');
  let certificateStatus = $state<CertificateStatus>('pending');
  let certificatesLoading = $state(false);
  let certificatesSubmitting = $state(false);
  let certificatesError = $state('');
  let currentRoute = $state<AdminRoute>(resolveAdminRoute(window.location.pathname));
  let dashboard = $state<DashboardState>(createEmptyDashboard());
  let componentStatuses = $state<ComponentConfigurationStatus[]>([]);
  let configurationLoading = $state(false);
  let configurationError = $state('');
  let summary = $derived(dashboardSummary(dashboard));

  onMount(() => {
    const handlePopState = () => {
      currentRoute = resolveAdminRoute(window.location.pathname);
    };

    window.addEventListener('popstate', handlePopState);
    void refreshSession();

    return () => {
      window.removeEventListener('popstate', handlePopState);
    };
  });

  function changeLocale(value: string): void {
    const nextLocale = resolveLocale(value);
    locale = nextLocale;
    persistLocale(nextLocale);
  }

  function persistLocale(nextLocale: Locale): void {
    document.documentElement.lang = nextLocale;
    localStorage.setItem(languageStorageKey, nextLocale);
  }

  function navigate(route: AdminRoute): void {
    const path = pathForRoute(route);

    if (window.location.pathname !== path) {
      window.history.pushState({}, '', path);
    }

    currentRoute = route;
  }

  function replacePath(path: string): void {
    if (window.location.pathname !== path) {
      window.history.replaceState({}, '', path);
    }

    currentRoute = resolveAdminRoute(path);
  }

  function syncAuthenticatedPath(): void {
    const redirectPath = shouldRedirectAuthenticatedUser(window.location.pathname);

    if (redirectPath) {
      replacePath(redirectPath);
    } else {
      currentRoute = resolveAdminRoute(window.location.pathname);
    }
  }

  function componentStatusLabel(status: ComponentStatus): string {
    if (status === 'configured') {
      return i18n.text('components.configured');
    }

    if (status === 'disabled') {
      return i18n.text('status.disabled');
    }

    return i18n.text('components.needsConfiguration');
  }

  function componentStatusClass(status: ComponentStatus): string {
    if (status === 'configured') {
      return 'text-bg-success';
    }

    if (status === 'disabled') {
      return 'text-bg-secondary';
    }

    return 'text-bg-warning';
  }

  async function loadConfiguration(): Promise<void> {
    configurationLoading = true;
    configurationError = '';
    try {
      const configuration = await configClient.load();
      componentStatuses = componentStatusesFromConfiguration(configuration);
    } catch {
      componentStatuses = [];
      configurationError = i18n.text('components.loadError');
    } finally {
      configurationLoading = false;
    }
  }

  async function loadApplications(): Promise<void> {
    applicationsLoading = true;
    applicationsError = '';
    try {
      dashboard.applications = await applicationsClient.list();
    } catch {
      dashboard.applications = [];
      applicationsError = i18n.text('applications.loadError');
    } finally {
      applicationsLoading = false;
    }
  }

  async function loadWafRules(): Promise<void> {
    wafRulesLoading = true;
    wafRulesError = '';
    try {
      dashboard.rules = await wafRulesClient.list();
    } catch {
      dashboard.rules = [];
      wafRulesError = i18n.text('rules.loadError');
    } finally {
      wafRulesLoading = false;
    }
  }

  async function loadRateLimits(): Promise<void> {
    rateLimitsLoading = true;
    rateLimitsError = '';
    try {
      dashboard.rateLimits = await rateLimitsClient.list();
    } catch {
      dashboard.rateLimits = [];
      rateLimitsError = i18n.text('rateLimits.loadError');
    } finally {
      rateLimitsLoading = false;
    }
  }

  async function loadDnsRecords(): Promise<void> {
    dnsLoading = true;
    dnsError = '';
    try {
      dashboard.dnsRecords = await dnsClient.listRecords();
    } catch {
      dashboard.dnsRecords = [];
      dnsError = i18n.text('dns.loadError');
    } finally {
      dnsLoading = false;
    }
  }

  async function loadAuditEvents(): Promise<void> {
    auditLoading = true;
    auditError = '';
    try {
      dashboard.auditEvents = await auditClient.list();
    } catch {
      dashboard.auditEvents = [];
      auditError = i18n.text('audit.loadError');
    } finally {
      auditLoading = false;
    }
  }

  async function loadCertificates(): Promise<void> {
    certificatesLoading = true;
    certificatesError = '';
    try {
      dashboard.certificates = await certificatesClient.list();
    } catch {
      dashboard.certificates = [];
      certificatesError = i18n.text('certificates.loadError');
    } finally {
      certificatesLoading = false;
    }
  }

  async function submitApplication(): Promise<void> {
    const csrfToken = authSession.csrfToken;

    if (!csrfToken) {
      applicationsError = i18n.text('applications.authRequired');
      return;
    }

    applicationsSubmitting = true;
    applicationsError = '';
    try {
      const application = await applicationsClient.create(csrfToken, {
        name: applicationName,
        hostname: applicationHostname,
        upstreamDial: applicationUpstreamDial
      });
      dashboard.applications = [...dashboard.applications, application];
      applicationName = '';
      applicationHostname = '';
      applicationUpstreamDial = '';
      await loadAuditEvents();
    } catch {
      applicationsError = i18n.text('applications.saveError');
    } finally {
      applicationsSubmitting = false;
    }
  }

  async function submitWafRule(): Promise<void> {
    const csrfToken = authSession.csrfToken;

    if (!csrfToken) {
      wafRulesError = i18n.text('rules.authRequired');
      return;
    }

    wafRulesSubmitting = true;
    wafRulesError = '';
    try {
      const rule = await wafRulesClient.create(csrfToken, {
        name: wafRuleName,
        applicationId: wafRuleApplicationId || null,
        priority: wafRulePriority,
        action: wafRuleAction,
        pathPrefix: wafRulePathPrefix || null
      });
      dashboard.rules = [...dashboard.rules, rule];
      wafRuleName = '';
      wafRuleApplicationId = '';
      wafRulePriority = 100;
      wafRuleAction = 'block';
      wafRulePathPrefix = '';
      await loadAuditEvents();
    } catch {
      wafRulesError = i18n.text('rules.saveError');
    } finally {
      wafRulesSubmitting = false;
    }
  }

  async function submitRateLimit(): Promise<void> {
    const csrfToken = authSession.csrfToken;

    if (!csrfToken) {
      rateLimitsError = i18n.text('rateLimits.authRequired');
      return;
    }

    rateLimitsSubmitting = true;
    rateLimitsError = '';
    try {
      const rateLimit = await rateLimitsClient.create(csrfToken, {
        name: rateLimitName,
        applicationId: rateLimitApplicationId || null,
        pathPrefix: rateLimitPathPrefix || null,
        requests: rateLimitRequests,
        windowSeconds: rateLimitWindowSeconds,
        action: rateLimitAction
      });
      dashboard.rateLimits = [...dashboard.rateLimits, rateLimit];
      rateLimitName = '';
      rateLimitApplicationId = '';
      rateLimitPathPrefix = '';
      rateLimitRequests = 100;
      rateLimitWindowSeconds = 60;
      rateLimitAction = 'block';
      await loadAuditEvents();
    } catch {
      rateLimitsError = i18n.text('rateLimits.saveError');
    } finally {
      rateLimitsSubmitting = false;
    }
  }

  async function submitDnsRecord(): Promise<void> {
    const csrfToken = authSession.csrfToken;

    if (!csrfToken) {
      dnsError = i18n.text('dns.authRequired');
      return;
    }

    dnsSubmitting = true;
    dnsError = '';
    try {
      const record = await dnsClient.createRecord(csrfToken, {
        zoneName: dnsZoneName,
        name: dnsRecordName,
        recordType: dnsRecordType,
        content: dnsRecordContent,
        ttl: dnsRecordTtl,
        proxied: dnsRecordProxied
      });
      dashboard.dnsRecords = [...dashboard.dnsRecords, record];
      dnsZoneName = '';
      dnsRecordName = '';
      dnsRecordType = 'A';
      dnsRecordContent = '';
      dnsRecordTtl = 300;
      dnsRecordProxied = false;
      await loadAuditEvents();
    } catch {
      dnsError = i18n.text('dns.saveError');
    } finally {
      dnsSubmitting = false;
    }
  }

  async function submitCertificate(): Promise<void> {
    const csrfToken = authSession.csrfToken;

    if (!csrfToken) {
      certificatesError = i18n.text('certificates.authRequired');
      return;
    }

    certificatesSubmitting = true;
    certificatesError = '';
    try {
      const certificate = await certificatesClient.create(csrfToken, {
        applicationId: certificateApplicationId || null,
        domain: certificateDomain,
        issuer: certificateIssuer,
        challengeType: certificateChallengeType,
        status: certificateStatus
      });
      dashboard.certificates = [...dashboard.certificates, certificate];
      certificateApplicationId = '';
      certificateDomain = '';
      certificateIssuer = 'letsencrypt';
      certificateChallengeType = 'http_01';
      certificateStatus = 'pending';
      await loadAuditEvents();
    } catch {
      certificatesError = i18n.text('certificates.saveError');
    } finally {
      certificatesSubmitting = false;
    }
  }

  async function refreshSession(): Promise<void> {
    authLoading = true;
    try {
      authSession = await auth.session();
      if (authSession.authenticated) {
        syncAuthenticatedPath();
        await Promise.all([loadConfiguration(), loadApplications(), loadWafRules(), loadRateLimits(), loadDnsRecords(), loadCertificates(), loadAuditEvents()]);
      } else if (window.location.pathname !== '/login') {
        replacePath('/login');
      }
    } catch {
      authSession = { authenticated: false, user: null, csrfToken: null };
      replacePath('/login');
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
      syncAuthenticatedPath();
      await Promise.all([loadConfiguration(), loadApplications(), loadWafRules(), loadRateLimits(), loadDnsRecords(), loadCertificates(), loadAuditEvents()]);
    } catch {
      loginError = i18n.text('auth.loginError');
    } finally {
      authSubmitting = false;
    }
  }

  async function submitLogout(): Promise<void> {
    authSession = await auth.logout();
    replacePath('/login');
  }

  async function submitPasswordChange(): Promise<void> {
    authSubmitting = true;
    changePasswordError = '';
    try {
      await auth.changePassword(currentPassword, newPassword);
      currentPassword = '';
      newPassword = '';
      authSession = await auth.session();
      syncAuthenticatedPath();
      await Promise.all([loadConfiguration(), loadApplications(), loadWafRules(), loadRateLimits(), loadDnsRecords(), loadCertificates(), loadAuditEvents()]);
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
        <div class="brand-lockup">
          <i class="bi bi-shield-lock fs-4"></i>
          <span class="fw-semibold">Baia WAF</span>
        </div>
        <div class="spinner-border mt-4" role="status" aria-label={i18n.text('auth.loading')}></div>
      </div>
    </section>
  {:else if !authSession.authenticated}
    <section class="auth-shell">
      <form class="auth-panel" onsubmit={(event) => { event.preventDefault(); void submitLogin(); }}>
        <div class="brand-lockup mb-4">
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
        <div class="brand-lockup mb-4">
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
    <div class="admin-shell">
      <aside class="admin-sidebar">
        <a class="admin-brand" href="/" onclick={(event) => { event.preventDefault(); navigate('overview'); }}>
          <i class="bi bi-shield-lock"></i>
          <span>Baia WAF</span>
        </a>
        <nav class="admin-nav" aria-label={i18n.text('nav.primary')}>
          {#each adminNavigation as item (item.route)}
            <a
              class:active={currentRoute === item.route}
              href={item.path}
              aria-current={currentRoute === item.route ? 'page' : undefined}
              onclick={(event) => { event.preventDefault(); navigate(item.route); }}
            >
              <i class={`bi ${item.icon}`}></i>
              <span>{i18n.text(item.labelKey)}</span>
            </a>
          {/each}
        </nav>
      </aside>

      <div class="admin-main">
        <header class="admin-topbar">
          <div>
            <p class="section-kicker mb-1">{i18n.text('app.kicker')}</p>
            <h1>{i18n.text(`page.${currentRoute}.title`)}</h1>
          </div>
          <div class="admin-actions">
            <label class="visually-hidden" for="locale-select">{i18n.text('nav.language')}</label>
            <select id="locale-select" class="form-select form-select-sm language-select" value={locale} aria-label={i18n.text('nav.language')} onchange={(event) => changeLocale(event.currentTarget.value)}>
              {#each supportedLocales as supportedLocale (supportedLocale.code)}
                <option value={supportedLocale.code}>{supportedLocale.label}</option>
              {/each}
            </select>
            <button class="icon-button" type="button" title={i18n.text('nav.applyCaddy')}>
              <i class="bi bi-arrow-repeat"></i>
            </button>
            <button class="btn btn-outline-secondary btn-sm" type="button" onclick={() => void submitLogout()}>
              <i class="bi bi-box-arrow-right me-1"></i>
              {i18n.text('auth.logout')}
            </button>
          </div>
        </header>

        <section class="content-shell">
          {#if currentRoute === 'overview'}
            <div class="metric-grid">
              <div class="metric-card">
                <span>{i18n.text('metrics.applications')}</span>
                <strong>{summary.applications}</strong>
              </div>
              <div class="metric-card">
                <span>{i18n.text('metrics.activeRules')}</span>
                <strong>{summary.activeRules}</strong>
              </div>
              <div class="metric-card">
                <span>{i18n.text('metrics.rateLimits')}</span>
                <strong>{summary.rateLimits}</strong>
              </div>
              <div class="metric-card">
                <span>{i18n.text('metrics.certificates')}</span>
                <strong>{summary.certificates}</strong>
              </div>
              <div class="metric-card">
                <span>{i18n.text('metrics.auditEvents')}</span>
                <strong>{summary.auditEvents}</strong>
              </div>
            </div>

            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('overview.nextStepTitle')}</h2>
                  <p>{i18n.text('overview.nextStepDescription')}</p>
                </div>
                <button class="btn btn-primary btn-sm" type="button" onclick={() => navigate('applications')}>
                  <i class="bi bi-plus-lg me-1"></i>
                  {i18n.text('applications.add')}
                </button>
              </div>
              <div class="empty-state">
                <i class="bi bi-window-plus"></i>
                <h3>{i18n.text('overview.emptyTitle')}</h3>
                <p>{i18n.text('overview.emptyDescription')}</p>
              </div>
            </div>

            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('certificates.title')}</h2>
                  <p>{i18n.text('certificates.description')}</p>
                </div>
                <button class="icon-button" type="button" title={i18n.text('services.refresh')} onclick={() => void loadCertificates()}>
                  <i class="bi bi-arrow-clockwise"></i>
                </button>
              </div>

              <form class="certificate-form" onsubmit={(event) => { event.preventDefault(); void submitCertificate(); }}>
                {#if certificatesError}
                  <div class="alert alert-danger mb-0" role="alert">{certificatesError}</div>
                {/if}
                <div>
                  <label class="form-label" for="certificate-application">{i18n.text('rules.application')}</label>
                  <select id="certificate-application" class="form-select" bind:value={certificateApplicationId}>
                    <option value="">{i18n.text('rules.global')}</option>
                    {#each dashboard.applications as application (application.id)}
                      <option value={application.id}>{application.name}</option>
                    {/each}
                  </select>
                </div>
                <div>
                  <label class="form-label" for="certificate-domain">{i18n.text('certificates.domain')}</label>
                  <input id="certificate-domain" class="form-control" bind:value={certificateDomain} required maxlength="253" />
                </div>
                <div>
                  <label class="form-label" for="certificate-issuer">{i18n.text('certificates.issuer')}</label>
                  <input id="certificate-issuer" class="form-control" bind:value={certificateIssuer} required maxlength="120" />
                </div>
                <div>
                  <label class="form-label" for="certificate-challenge">{i18n.text('certificates.challenge')}</label>
                  <select id="certificate-challenge" class="form-select" bind:value={certificateChallengeType}>
                    <option value="http_01">HTTP-01</option>
                    <option value="dns_01">DNS-01</option>
                  </select>
                </div>
                <div>
                  <label class="form-label" for="certificate-status">{i18n.text('certificates.status')}</label>
                  <select id="certificate-status" class="form-select" bind:value={certificateStatus}>
                    <option value="pending">{i18n.text('certificates.statusPending')}</option>
                    <option value="issued">{i18n.text('certificates.statusIssued')}</option>
                    <option value="failed">{i18n.text('certificates.statusFailed')}</option>
                    <option value="revoked">{i18n.text('certificates.statusRevoked')}</option>
                  </select>
                </div>
                <div class="certificate-form-action">
                  <button class="btn btn-primary" type="submit" disabled={certificatesSubmitting}>
                    <i class="bi bi-plus-lg me-1"></i>
                    {certificatesSubmitting ? i18n.text('certificates.saving') : i18n.text('certificates.add')}
                  </button>
                </div>
              </form>

              {#if certificatesLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('certificates.loading')}></div>
                </div>
              {:else if dashboard.certificates.length === 0}
                <div class="empty-state">
                  <i class="bi bi-patch-check"></i>
                  <h3>{i18n.text('certificates.emptyTitle')}</h3>
                  <p>{i18n.text('certificates.emptyDescription')}</p>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">{i18n.text('certificates.domain')}</th>
                        <th scope="col">{i18n.text('rules.application')}</th>
                        <th scope="col">{i18n.text('certificates.issuer')}</th>
                        <th scope="col">{i18n.text('certificates.challenge')}</th>
                        <th scope="col">{i18n.text('certificates.status')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each dashboard.certificates as certificate (certificate.id)}
                        <tr>
                          <th scope="row">{certificate.domain}</th>
                          <td>{certificate.applicationName ?? i18n.text('rules.global')}</td>
                          <td>{certificate.issuer}</td>
                          <td>{certificate.challengeType}</td>
                          <td>{certificate.status}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>

            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('components.configurationTitle')}</h2>
                  <p>{i18n.text('components.configurationDescription')}</p>
                </div>
                <button class="icon-button" type="button" title={i18n.text('services.refresh')} onclick={() => void loadConfiguration()}>
                  <i class="bi bi-arrow-clockwise"></i>
                </button>
              </div>
              {#if configurationLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('components.loading')}></div>
                </div>
              {:else if configurationError}
                <div class="empty-state">
                  <i class="bi bi-exclamation-triangle"></i>
                  <h3>{configurationError}</h3>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">{i18n.text('table.service')}</th>
                        <th scope="col">{i18n.text('table.status')}</th>
                        <th scope="col">{i18n.text('table.detail')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each componentStatuses as component (component.id)}
                        <tr>
                          <th scope="row">{component.name}</th>
                          <td><span class={`badge ${componentStatusClass(component.status)}`}>{componentStatusLabel(component.status)}</span></td>
                          <td>{i18n.text(component.detailKey)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {:else if currentRoute === 'applications'}
            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('applications.title')}</h2>
                  <p>{i18n.text('applications.description')}</p>
                </div>
              </div>

              <form class="application-form" onsubmit={(event) => { event.preventDefault(); void submitApplication(); }}>
                {#if applicationsError}
                  <div class="alert alert-danger mb-0" role="alert">{applicationsError}</div>
                {/if}
                <div>
                  <label class="form-label" for="application-name">{i18n.text('applications.name')}</label>
                  <input id="application-name" class="form-control" bind:value={applicationName} required maxlength="120" />
                </div>
                <div>
                  <label class="form-label" for="application-hostname">{i18n.text('applications.hostname')}</label>
                  <input id="application-hostname" class="form-control" bind:value={applicationHostname} required maxlength="253" />
                </div>
                <div>
                  <label class="form-label" for="application-upstream">{i18n.text('applications.upstream')}</label>
                  <input id="application-upstream" class="form-control" bind:value={applicationUpstreamDial} required maxlength="255" />
                </div>
                <div class="application-form-action">
                  <button class="btn btn-primary" type="submit" disabled={applicationsSubmitting}>
                    <i class="bi bi-plus-lg me-1"></i>
                    {applicationsSubmitting ? i18n.text('applications.saving') : i18n.text('applications.add')}
                  </button>
                </div>
              </form>

              {#if applicationsLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('applications.loading')}></div>
                </div>
              {:else if dashboard.applications.length === 0}
                <div class="empty-state">
                  <i class="bi bi-window-stack"></i>
                  <h3>{i18n.text('applications.emptyTitle')}</h3>
                  <p>{i18n.text('applications.emptyDescription')}</p>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">{i18n.text('applications.name')}</th>
                        <th scope="col">{i18n.text('applications.hostname')}</th>
                        <th scope="col">{i18n.text('applications.upstream')}</th>
                        <th scope="col">{i18n.text('applications.status')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each dashboard.applications as application (application.id)}
                        <tr>
                          <th scope="row">{application.name}</th>
                          <td>{application.hostname}</td>
                          <td>{application.upstreams[0]?.dial ?? ''}</td>
                          <td>{application.enabled ? i18n.text('status.enabled') : i18n.text('status.disabled')}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {:else if currentRoute === 'rules'}
            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('rules.title')}</h2>
                  <p>{i18n.text('rules.description')}</p>
                </div>
              </div>

              <form class="rule-form" onsubmit={(event) => { event.preventDefault(); void submitWafRule(); }}>
                {#if wafRulesError}
                  <div class="alert alert-danger mb-0" role="alert">{wafRulesError}</div>
                {/if}
                <div>
                  <label class="form-label" for="waf-rule-name">{i18n.text('rules.name')}</label>
                  <input id="waf-rule-name" class="form-control" bind:value={wafRuleName} required maxlength="120" />
                </div>
                <div>
                  <label class="form-label" for="waf-rule-application">{i18n.text('rules.application')}</label>
                  <select id="waf-rule-application" class="form-select" bind:value={wafRuleApplicationId}>
                    <option value="">{i18n.text('rules.global')}</option>
                    {#each dashboard.applications as application (application.id)}
                      <option value={application.id}>{application.name}</option>
                    {/each}
                  </select>
                </div>
                <div>
                  <label class="form-label" for="waf-rule-priority">{i18n.text('rules.priority')}</label>
                  <input id="waf-rule-priority" class="form-control" type="number" min="0" max="100000" bind:value={wafRulePriority} required />
                </div>
                <div>
                  <label class="form-label" for="waf-rule-action">{i18n.text('rules.action')}</label>
                  <select id="waf-rule-action" class="form-select" bind:value={wafRuleAction}>
                    <option value="allow">{i18n.text('rules.actionAllow')}</option>
                    <option value="block">{i18n.text('rules.actionBlock')}</option>
                    <option value="challenge">{i18n.text('rules.actionChallenge')}</option>
                    <option value="rate_limit">{i18n.text('rules.actionRateLimit')}</option>
                    <option value="log">{i18n.text('rules.actionLog')}</option>
                  </select>
                </div>
                <div>
                  <label class="form-label" for="waf-rule-path">{i18n.text('rules.pathPrefix')}</label>
                  <input id="waf-rule-path" class="form-control" bind:value={wafRulePathPrefix} maxlength="256" />
                </div>
                <div class="rule-form-action">
                  <button class="btn btn-primary" type="submit" disabled={wafRulesSubmitting}>
                    <i class="bi bi-plus-lg me-1"></i>
                    {wafRulesSubmitting ? i18n.text('rules.saving') : i18n.text('rules.new')}
                  </button>
                </div>
              </form>

              {#if wafRulesLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('rules.loading')}></div>
                </div>
              {:else if dashboard.rules.length === 0}
                <div class="empty-state">
                  <i class="bi bi-shield-plus"></i>
                  <h3>{i18n.text('rules.emptyTitle')}</h3>
                  <p>{i18n.text('rules.emptyDescription')}</p>
                </div>
              {:else}
                <div class="list-group list-group-flush">
                  {#each dashboard.rules as rule (rule.id)}
                    <div class="list-group-item">
                      <div class="d-flex justify-content-between gap-3">
                        <div>
                          <div class="fw-semibold">{rule.priority}. {rule.name}</div>
                          <div class="text-body-secondary small">{rule.applicationName ?? i18n.text('rules.global')}{rule.pathPrefix ? ` · ${rule.pathPrefix}` : ''}</div>
                        </div>
                        <span class="badge text-bg-info align-self-start">{i18n.text(`rules.action.${rule.action}`)}</span>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {:else if currentRoute === 'rate-limits'}
            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('rateLimits.title')}</h2>
                  <p>{i18n.text('rateLimits.description')}</p>
                </div>
                <button class="icon-button" type="button" title={i18n.text('services.refresh')} onclick={() => void loadRateLimits()}>
                  <i class="bi bi-arrow-clockwise"></i>
                </button>
              </div>

              <form class="rate-limit-form" onsubmit={(event) => { event.preventDefault(); void submitRateLimit(); }}>
                {#if rateLimitsError}
                  <div class="alert alert-danger mb-0" role="alert">{rateLimitsError}</div>
                {/if}
                <div>
                  <label class="form-label" for="rate-limit-name">{i18n.text('rateLimits.name')}</label>
                  <input id="rate-limit-name" class="form-control" bind:value={rateLimitName} required maxlength="120" />
                </div>
                <div>
                  <label class="form-label" for="rate-limit-application">{i18n.text('rules.application')}</label>
                  <select id="rate-limit-application" class="form-select" bind:value={rateLimitApplicationId}>
                    <option value="">{i18n.text('rules.global')}</option>
                    {#each dashboard.applications as application (application.id)}
                      <option value={application.id}>{application.name}</option>
                    {/each}
                  </select>
                </div>
                <div>
                  <label class="form-label" for="rate-limit-path">{i18n.text('rules.pathPrefix')}</label>
                  <input id="rate-limit-path" class="form-control" bind:value={rateLimitPathPrefix} maxlength="256" />
                </div>
                <div>
                  <label class="form-label" for="rate-limit-requests">{i18n.text('rateLimits.requests')}</label>
                  <input id="rate-limit-requests" class="form-control" type="number" min="1" max="1000000" bind:value={rateLimitRequests} required />
                </div>
                <div>
                  <label class="form-label" for="rate-limit-window">{i18n.text('rateLimits.window')}</label>
                  <input id="rate-limit-window" class="form-control" type="number" min="10" max="86400" bind:value={rateLimitWindowSeconds} required />
                </div>
                <div>
                  <label class="form-label" for="rate-limit-action">{i18n.text('rateLimits.action')}</label>
                  <select id="rate-limit-action" class="form-select" bind:value={rateLimitAction}>
                    <option value="block">{i18n.text('rateLimits.actionBlock')}</option>
                    <option value="challenge">{i18n.text('rateLimits.actionChallenge')}</option>
                    <option value="log">{i18n.text('rateLimits.actionLog')}</option>
                  </select>
                </div>
                <div class="rate-limit-form-action">
                  <button class="btn btn-primary" type="submit" disabled={rateLimitsSubmitting}>
                    <i class="bi bi-plus-lg me-1"></i>
                    {rateLimitsSubmitting ? i18n.text('rateLimits.saving') : i18n.text('rateLimits.new')}
                  </button>
                </div>
              </form>

              {#if rateLimitsLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('rateLimits.loading')}></div>
                </div>
              {:else if dashboard.rateLimits.length === 0}
                <div class="empty-state">
                  <i class="bi bi-stopwatch"></i>
                  <h3>{i18n.text('rateLimits.emptyTitle')}</h3>
                  <p>{i18n.text('rateLimits.emptyDescription')}</p>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">{i18n.text('rateLimits.name')}</th>
                        <th scope="col">{i18n.text('rules.application')}</th>
                        <th scope="col">{i18n.text('rules.pathPrefix')}</th>
                        <th scope="col">{i18n.text('rateLimits.requests')}</th>
                        <th scope="col">{i18n.text('rateLimits.window')}</th>
                        <th scope="col">{i18n.text('rateLimits.action')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each dashboard.rateLimits as limit (limit.id)}
                        <tr>
                          <th scope="row">{limit.name}</th>
                          <td>{limit.applicationName ?? i18n.text('rules.global')}</td>
                          <td>{limit.pathPrefix ?? '*'}</td>
                          <td>{limit.requests}</td>
                          <td>{limit.windowSeconds}s</td>
                          <td><span class="badge text-bg-info">{i18n.text(`rateLimits.action.${limit.action}`)}</span></td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {:else if currentRoute === 'dns'}
            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('dns.title')}</h2>
                  <p>{i18n.text('dns.description')}</p>
                </div>
              </div>

              <form class="dns-form" onsubmit={(event) => { event.preventDefault(); void submitDnsRecord(); }}>
                {#if dnsError}
                  <div class="alert alert-danger mb-0" role="alert">{dnsError}</div>
                {/if}
                <div>
                  <label class="form-label" for="dns-zone">{i18n.text('dns.zone')}</label>
                  <input id="dns-zone" class="form-control" bind:value={dnsZoneName} required maxlength="253" />
                </div>
                <div>
                  <label class="form-label" for="dns-name">{i18n.text('dns.name')}</label>
                  <input id="dns-name" class="form-control" bind:value={dnsRecordName} required maxlength="253" />
                </div>
                <div>
                  <label class="form-label" for="dns-type">Type</label>
                  <select id="dns-type" class="form-select" bind:value={dnsRecordType}>
                    <option value="A">A</option>
                    <option value="AAAA">AAAA</option>
                    <option value="CNAME">CNAME</option>
                    <option value="TXT">TXT</option>
                    <option value="CAA">CAA</option>
                    <option value="MX">MX</option>
                  </select>
                </div>
                <div>
                  <label class="form-label" for="dns-content">Value</label>
                  <input id="dns-content" class="form-control" bind:value={dnsRecordContent} required maxlength="2048" />
                </div>
                <div>
                  <label class="form-label" for="dns-ttl">TTL</label>
                  <input id="dns-ttl" class="form-control" type="number" min="60" max="86400" bind:value={dnsRecordTtl} required />
                </div>
                <div class="form-check dns-proxy-check">
                  <input id="dns-proxied" class="form-check-input" type="checkbox" bind:checked={dnsRecordProxied} />
                  <label class="form-check-label" for="dns-proxied">{i18n.text('dns.proxy')}</label>
                </div>
                <div class="dns-form-action">
                  <button class="btn btn-primary" type="submit" disabled={dnsSubmitting}>
                    <i class="bi bi-plus-lg me-1"></i>
                    {dnsSubmitting ? i18n.text('dns.saving') : i18n.text('dns.add')}
                  </button>
                </div>
              </form>

              {#if dnsLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('dns.loading')}></div>
                </div>
              {:else if dashboard.dnsRecords.length === 0}
                <div class="empty-state">
                  <i class="bi bi-diagram-3"></i>
                  <h3>{i18n.text('dns.emptyTitle')}</h3>
                  <p>{i18n.text('dns.emptyDescription')}</p>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">Type</th>
                        <th scope="col">{i18n.text('dns.zone')}</th>
                        <th scope="col">{i18n.text('dns.name')}</th>
                        <th scope="col">Value</th>
                        <th scope="col">TTL</th>
                        <th scope="col">{i18n.text('dns.proxy')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each dashboard.dnsRecords as record (record.id)}
                        <tr>
                          <td>{record.recordType}</td>
                          <td>{record.zoneName}</td>
                          <td>{record.name}</td>
                          <td><code>{record.content}</code></td>
                          <td>{record.ttl}</td>
                          <td>{record.proxied ? i18n.text('dns.proxied') : i18n.text('dns.dnsOnly')}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {:else}
            <div class="workspace-panel">
              <div class="panel-heading">
                <div>
                  <h2>{i18n.text('audit.title')}</h2>
                  <p>{i18n.text('audit.description')}</p>
                </div>
                <button class="icon-button" type="button" title={i18n.text('services.refresh')} onclick={() => void loadAuditEvents()}>
                  <i class="bi bi-arrow-clockwise"></i>
                </button>
              </div>
              {#if auditLoading}
                <div class="empty-state">
                  <div class="spinner-border" role="status" aria-label={i18n.text('audit.loading')}></div>
                </div>
              {:else if auditError}
                <div class="empty-state">
                  <i class="bi bi-exclamation-triangle"></i>
                  <h3>{auditError}</h3>
                </div>
              {:else if dashboard.auditEvents.length === 0}
                <div class="empty-state">
                  <i class="bi bi-clock-history"></i>
                  <h3>{i18n.text('audit.emptyTitle')}</h3>
                  <p>{i18n.text('audit.emptyDescription')}</p>
                </div>
              {:else}
                <div class="table-responsive">
                  <table class="table align-middle mb-0">
                    <thead>
                      <tr>
                        <th scope="col">{i18n.text('audit.when')}</th>
                        <th scope="col">{i18n.text('audit.actor')}</th>
                        <th scope="col">{i18n.text('audit.action')}</th>
                        <th scope="col">{i18n.text('audit.resource')}</th>
                        <th scope="col">{i18n.text('audit.result')}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each dashboard.auditEvents as event (event.id)}
                        <tr>
                          <td>{event.occurredAt}</td>
                          <td>{event.actor}</td>
                          <td>{event.action}</td>
                          <td>{event.resourceType}:{event.resourceId}</td>
                          <td>{event.result}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          {/if}
        </section>
      </div>
    </div>
  {/if}
</main>

<style>
  :global(body) {
    min-width: 320px;
  }

  .brand-lockup {
    display: flex;
    align-items: center;
    gap: .75rem;
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

  .admin-shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 16rem minmax(0, 1fr);
    background: #f5f7fb;
  }

  .admin-sidebar {
    background: #111827;
    color: #e5e7eb;
    padding: 1.25rem;
    border-right: 1px solid rgba(255, 255, 255, .08);
  }

  .admin-brand {
    display: flex;
    align-items: center;
    gap: .75rem;
    color: #ffffff;
    text-decoration: none;
    font-weight: 700;
    padding: .75rem .5rem 1.25rem;
  }

  .admin-nav {
    display: grid;
    gap: .25rem;
  }

  .admin-nav a {
    display: flex;
    align-items: center;
    gap: .75rem;
    color: #aeb7c7;
    text-decoration: none;
    border-radius: .5rem;
    padding: .75rem;
    font-weight: 600;
  }

  .admin-nav a:hover,
  .admin-nav a.active {
    background: #243044;
    color: #ffffff;
  }

  .admin-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .admin-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    min-height: 5.5rem;
    padding: 1.25rem 1.5rem;
    background: #ffffff;
    border-bottom: 1px solid #dfe4ec;
  }

  .admin-topbar h1 {
    font-size: 1.35rem;
    line-height: 1.2;
    margin: 0;
  }

  .section-kicker {
    color: #667085;
    font-size: .78rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .admin-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: .5rem;
    flex-wrap: wrap;
  }

  .language-select {
    min-width: 8.5rem;
  }

  .icon-button {
    width: 2rem;
    height: 2rem;
    display: inline-grid;
    place-items: center;
    border: 1px solid #cfd6e3;
    border-radius: .375rem;
    background: #ffffff;
    color: #344054;
  }

  .content-shell {
    padding: 1.5rem;
    display: grid;
    gap: 1rem;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
    gap: 1rem;
  }

  .metric-card,
  .workspace-panel {
    background: #ffffff;
    border: 1px solid #dfe4ec;
    border-radius: .5rem;
  }

  .metric-card {
    padding: 1rem;
    display: grid;
    gap: .5rem;
  }

  .metric-card span {
    color: #667085;
    font-size: .875rem;
    font-weight: 600;
  }

  .metric-card strong {
    color: #101828;
    font-size: 2rem;
    line-height: 1;
  }

  .panel-heading {
    min-height: 4.5rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem;
    border-bottom: 1px solid #e6ebf2;
  }

  .panel-heading h2 {
    font-size: 1rem;
    margin: 0;
  }

  .panel-heading p {
    color: #667085;
    margin: .25rem 0 0;
  }

  .application-form,
  .rule-form,
  .rate-limit-form,
  .dns-form,
  .certificate-form {
    padding: 1rem;
    display: grid;
    gap: 1rem;
    align-items: end;
    border-bottom: 1px solid #e6ebf2;
  }

  .application-form {
    grid-template-columns: minmax(10rem, 1fr) minmax(12rem, 1.2fr) minmax(12rem, 1.2fr) auto;
  }

  .rule-form {
    grid-template-columns: minmax(10rem, 1fr) minmax(10rem, 1fr) 8rem 10rem minmax(8rem, 1fr) auto;
  }

  .rate-limit-form {
    grid-template-columns: minmax(10rem, 1fr) minmax(10rem, 1fr) minmax(8rem, 1fr) 7rem 7rem 8rem auto;
  }

  .dns-form {
    grid-template-columns: minmax(10rem, 1fr) minmax(12rem, 1fr) 7rem minmax(12rem, 1.4fr) 7rem 7rem auto;
  }

  .certificate-form {
    grid-template-columns: minmax(10rem, 1fr) minmax(12rem, 1fr) minmax(10rem, 1fr) 9rem 9rem auto;
  }

  .application-form .alert,
  .rule-form .alert,
  .rate-limit-form .alert,
  .dns-form .alert,
  .certificate-form .alert {
    grid-column: 1 / -1;
  }

  .application-form-action,
  .rule-form-action,
  .rate-limit-form-action,
  .dns-form-action,
  .certificate-form-action {
    display: flex;
    justify-content: flex-end;
  }

  .dns-proxy-check {
    min-height: 2.375rem;
    display: flex;
    align-items: center;
    gap: .5rem;
  }

  .empty-state {
    min-height: 18rem;
    display: grid;
    place-items: center;
    align-content: center;
    gap: .75rem;
    text-align: center;
    padding: 2rem;
    color: #667085;
  }

  .empty-state i {
    font-size: 2rem;
    color: #3b82f6;
  }

  .empty-state h3 {
    margin: 0;
    color: #101828;
    font-size: 1.1rem;
  }

  .empty-state p {
    max-width: 35rem;
    margin: 0;
  }

  @media (max-width: 991.98px) {
    .admin-shell {
      grid-template-columns: 1fr;
    }

    .admin-sidebar {
      position: static;
      padding: .75rem;
    }

    .admin-brand {
      padding: .5rem;
    }

    .admin-nav {
      display: flex;
      overflow-x: auto;
      padding-bottom: .25rem;
    }

    .admin-nav a {
      flex: 0 0 auto;
    }

    .admin-topbar {
      align-items: flex-start;
      flex-direction: column;
    }

    .admin-actions {
      justify-content: flex-start;
      width: 100%;
    }

    .metric-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .application-form,
    .rule-form,
    .rate-limit-form,
    .dns-form,
    .certificate-form {
      grid-template-columns: 1fr;
    }

    .application-form-action,
    .rule-form-action,
    .rate-limit-form-action,
    .dns-form-action,
    .certificate-form-action {
      justify-content: flex-start;
    }
  }

  @media (max-width: 575.98px) {
    .content-shell {
      padding: 1rem;
    }

    .metric-grid {
      grid-template-columns: 1fr;
    }

    .panel-heading {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
