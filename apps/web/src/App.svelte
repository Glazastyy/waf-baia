<script lang="ts">
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

  const languageStorageKey = 'baia.locale';
  const initialLocale = resolveLocale(localStorage.getItem(languageStorageKey) ?? navigator.language);
  persistLocale(initialLocale);

  let locale = $state<Locale>(initialLocale);
  let i18n = $derived(localize(locale));

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

  function persistLocale(nextLocale: Locale): void {
    document.documentElement.lang = nextLocale;
    localStorage.setItem(languageStorageKey, nextLocale);
  }
</script>

<main class="min-vh-100 bg-body-tertiary">
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
</style>
