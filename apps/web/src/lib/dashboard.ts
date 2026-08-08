import type { Application } from './applications';
import type { AuditEvent } from './audit';
import type { Certificate } from './certificates';
import type { DnsRecord } from './dns';
import type { RateLimit } from './rate-limits';
import type { WafRule } from './waf-rules';

export type { Application } from './applications';
export type { AuditEvent } from './audit';
export type { Certificate } from './certificates';
export type { DnsRecord } from './dns';
export type { RateLimit } from './rate-limits';
export type { WafRule } from './waf-rules';

export type DashboardState = {
  applications: Application[];
  rules: WafRule[];
  rateLimits: RateLimit[];
  certificates: Certificate[];
  dnsRecords: DnsRecord[];
  auditEvents: AuditEvent[];
};

export type DashboardSummary = {
  applications: number;
  activeRules: number;
  rateLimits: number;
  certificates: number;
  dnsRecords: number;
  auditEvents: number;
};

export function createEmptyDashboard(): DashboardState {
  return {
    applications: [],
    rules: [],
    rateLimits: [],
    certificates: [],
    dnsRecords: [],
    auditEvents: []
  };
}

export function dashboardSummary(dashboard: DashboardState): DashboardSummary {
  return {
    applications: dashboard.applications.length,
    activeRules: dashboard.rules.filter((rule) => rule.enabled).length,
    rateLimits: dashboard.rateLimits.filter((limit) => limit.enabled).length,
    certificates: dashboard.certificates.length,
    dnsRecords: dashboard.dnsRecords.length,
    auditEvents: dashboard.auditEvents.length
  };
}
