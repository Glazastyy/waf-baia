<script lang="ts">
  type ServiceStatus = 'healthy' | 'degraded' | 'disabled';

  type Service = {
    name: string;
    role: string;
    status: ServiceStatus;
    detail: string;
  };

  type Rule = {
    priority: number;
    name: string;
    scope: string;
    action: string;
    enabled: boolean;
  };

  type Certificate = {
    domain: string;
    issuer: string;
    status: string;
    renewal: string;
  };

  let services = $state<Service[]>([
    { name: 'Core API', role: 'Admin API, auth, config orchestration', status: 'healthy', detail: 'Ready' },
    { name: 'Caddy', role: 'Reverse proxy and WAF enforcement', status: 'healthy', detail: 'Admin API reachable' },
    { name: 'PostgreSQL', role: 'Persistent control plane database', status: 'healthy', detail: 'Migrations current' },
    { name: 'Redis', role: 'Sessions, locks, cache and rate limits', status: 'healthy', detail: 'Distributed mode' },
    { name: 'CrowdSec', role: 'Reputation and hostile IP decisions', status: 'disabled', detail: 'Module disabled' },
    { name: 'PowerDNS', role: 'Integrated authoritative DNS', status: 'degraded', detail: 'API key missing' }
  ]);

  let rules = $state<Rule[]>([
    { priority: 10, name: 'Block admin scanners', scope: 'Path /wp-admin from unknown users', action: 'Block', enabled: true },
    { priority: 20, name: 'API write throttle', scope: 'POST /api by IP and API key', action: 'Rate limit', enabled: true },
    { priority: 30, name: 'Risk challenge', scope: 'Low reputation IPs', action: 'Challenge', enabled: false }
  ]);

  let certificates = $state<Certificate[]>([
    { domain: 'admin.waf.localhost', issuer: 'ACME HTTP-01', status: 'Issued', renewal: '21 days' },
    { domain: '*.example.test', issuer: 'ACME DNS-01', status: 'Pending DNS provider', renewal: 'Unavailable' }
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
          <li class="nav-item"><a class="nav-link active" aria-current="page" href="/">Overview</a></li>
          <li class="nav-item"><a class="nav-link" href="/applications">Applications</a></li>
          <li class="nav-item"><a class="nav-link" href="/rules">Rules</a></li>
          <li class="nav-item"><a class="nav-link" href="/dns">DNS</a></li>
          <li class="nav-item"><a class="nav-link" href="/audit">Audit</a></li>
        </ul>
        <div class="d-flex gap-2">
          <button class="btn btn-outline-light btn-sm" type="button" title="Apply Caddy configuration">
            <i class="bi bi-arrow-repeat"></i>
          </button>
          <button class="btn btn-warning btn-sm" type="button">
            <i class="bi bi-exclamation-triangle me-1"></i>
            2 actions
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
              <h1 class="h5 mb-0">Control Plane</h1>
              <span class="badge text-bg-success">Online</span>
            </div>
            <p class="text-body-secondary mb-0 mt-2">6 services, 3 active security rules, 2 certificate workflows.</p>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">Protected hosts</div>
            <div class="display-6">12</div>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">Blocked requests</div>
            <div class="display-6">1,284</div>
          </div>
        </div>
      </div>
      <div class="col-12 col-md-4 col-xl-3">
        <div class="card shadow-sm h-100">
          <div class="card-body">
            <div class="text-body-secondary small">P95 upstream latency</div>
            <div class="display-6">82 ms</div>
          </div>
        </div>
      </div>
    </div>

    <div class="row g-4">
      <section class="col-12 col-xxl-7">
        <div class="card shadow-sm">
          <div class="card-header d-flex align-items-center justify-content-between">
            <span class="fw-semibold">Services</span>
            <button class="btn btn-outline-secondary btn-sm" type="button" title="Refresh services">
              <i class="bi bi-arrow-clockwise"></i>
            </button>
          </div>
          <div class="table-responsive">
            <table class="table table-hover align-middle mb-0">
              <thead>
                <tr>
                  <th scope="col">Service</th>
                  <th scope="col">Responsibility</th>
                  <th scope="col">Status</th>
                  <th scope="col">Detail</th>
                </tr>
              </thead>
              <tbody>
                {#each services as service (service.name)}
                  <tr>
                    <th scope="row">{service.name}</th>
                    <td>{service.role}</td>
                    <td><span class={`badge ${statusClass(service.status)}`}>{service.status}</span></td>
                    <td>{service.detail}</td>
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
            <span class="fw-semibold">WAF Rules</span>
            <button class="btn btn-primary btn-sm" type="button">
              <i class="bi bi-plus-lg me-1"></i>
              Rule
            </button>
          </div>
          <div class="list-group list-group-flush">
            {#each rules as rule (rule.name)}
              <div class="list-group-item">
                <div class="d-flex justify-content-between gap-3">
                  <div>
                    <div class="fw-semibold">{rule.priority}. {rule.name}</div>
                    <div class="text-body-secondary small">{rule.scope}</div>
                  </div>
                  <div class="d-flex align-items-center gap-2">
                    <span class="badge text-bg-info">{rule.action}</span>
                    <div class="form-check form-switch mb-0">
                      <input class="form-check-input" type="checkbox" checked={rule.enabled} aria-label={`Toggle ${rule.name}`} onchange={() => toggleRule(rule)} />
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
          <div class="card-header fw-semibold">Certificates</div>
          <div class="table-responsive">
            <table class="table align-middle mb-0">
              <thead>
                <tr>
                  <th scope="col">Domain</th>
                  <th scope="col">Issuer</th>
                  <th scope="col">Status</th>
                  <th scope="col">Renewal</th>
                </tr>
              </thead>
              <tbody>
                {#each certificates as certificate (certificate.domain)}
                  <tr>
                    <th scope="row">{certificate.domain}</th>
                    <td>{certificate.issuer}</td>
                    <td>{certificate.status}</td>
                    <td>{certificate.renewal}</td>
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
</style>
