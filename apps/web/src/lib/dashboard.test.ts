import { describe, expect, test } from 'bun:test';
import { createEmptyDashboard, dashboardSummary } from './dashboard';

describe('dashboard state', () => {
  test('starts without sample applications, rules, certificates or dns records', () => {
    const dashboard = createEmptyDashboard();

    expect(dashboard.applications).toEqual([]);
    expect(dashboard.rules).toEqual([]);
    expect(dashboard.certificates).toEqual([]);
    expect(dashboard.dnsRecords).toEqual([]);
    expect(dashboard.auditEvents).toEqual([]);
  });

  test('derives summary counters from real dashboard collections', () => {
    const dashboard = createEmptyDashboard();

    expect(dashboardSummary(dashboard)).toEqual({
      applications: 0,
      activeRules: 0,
      certificates: 0,
      dnsRecords: 0,
      auditEvents: 0
    });
  });
});
