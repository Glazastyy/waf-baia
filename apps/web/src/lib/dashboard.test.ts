import { describe, expect, test } from 'bun:test';
import { createEmptyDashboard, dashboardSummary, filterDashboard } from './dashboard';

describe('dashboard state', () => {
  test('starts without sample applications, rules, rate limits, certificates or dns records', () => {
    const dashboard = createEmptyDashboard();

    expect(dashboard.applications).toEqual([]);
    expect(dashboard.rules).toEqual([]);
    expect(dashboard.rateLimits).toEqual([]);
    expect(dashboard.certificates).toEqual([]);
    expect(dashboard.dnsRecords).toEqual([]);
    expect(dashboard.auditEvents).toEqual([]);
  });

  test('derives summary counters from real dashboard collections', () => {
    const dashboard = createEmptyDashboard();

    expect(dashboardSummary(dashboard)).toEqual({
      applications: 0,
      activeRules: 0,
      rateLimits: 0,
      certificates: 0,
      dnsRecords: 0,
      auditEvents: 0
    });
  });

  test('filters real dashboard collections by visible operational fields', () => {
    const dashboard = createEmptyDashboard();
    dashboard.applications = [
      { id: 'app_1', name: 'Portal', hostname: 'portal.example.com', enabled: true, upstreams: [{ id: 'up_1', dial: '10.0.0.20:8080', weight: 100, enabled: true }] }
    ];
    dashboard.rules = [
      { id: 'rule_1', name: 'Block Admin', applicationId: 'app_1', applicationName: 'Portal', priority: 10, action: 'block', pathPrefix: '/admin', enabled: true }
    ];
    dashboard.rateLimits = [
      { id: 'limit_1', name: 'Login Burst', applicationId: 'app_1', applicationName: 'Portal', pathPrefix: '/login', requests: 20, windowSeconds: 60, action: 'block', enabled: true }
    ];
    dashboard.dnsRecords = [
      { id: 'dns_1', zoneId: 'zone_1', zoneName: 'example.com', name: 'portal.example.com', recordType: 'A', content: '10.0.0.20', ttl: 300, proxied: false }
    ];
    dashboard.certificates = [
      { id: 'cert_1', applicationId: 'app_1', applicationName: 'Portal', domain: 'portal.example.com', issuer: 'letsencrypt', challengeType: 'http_01', status: 'pending' }
    ];
    dashboard.auditEvents = [
      { id: 'audit_1', actor: 'admin', action: 'application.create', resourceType: 'application', resourceId: 'app_1', result: 'success', occurredAt: '1' }
    ];

    const filtered = filterDashboard(dashboard, 'login');

    expect(filtered.applications).toEqual([]);
    expect(filtered.rules).toEqual([]);
    expect(filtered.rateLimits.map((limit) => limit.id)).toEqual(['limit_1']);
    expect(filtered.dnsRecords).toEqual([]);
    expect(filtered.certificates).toEqual([]);
    expect(filtered.auditEvents).toEqual([]);
  });
});
