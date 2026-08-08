import type { Application } from './applications';
import type { AuditEvent } from './audit';
import type { DnsRecord } from './dns';
import type { WafRule } from './waf-rules';

export type { Application } from './applications';
export type { AuditEvent } from './audit';
export type { DnsRecord } from './dns';
export type { WafRule } from './waf-rules';

export type Certificate = {
  id: string;
  domain: string;
  issuer: string;
  status: string;
  renewal: string | null;
};

export type DashboardState = {
  applications: Application[];
  rules: WafRule[];
  certificates: Certificate[];
  dnsRecords: DnsRecord[];
  auditEvents: AuditEvent[];
};

export type DashboardSummary = {
  applications: number;
  activeRules: number;
  certificates: number;
  dnsRecords: number;
  auditEvents: number;
};

export function createEmptyDashboard(): DashboardState {
  return {
    applications: [],
    rules: [],
    certificates: [],
    dnsRecords: [],
    auditEvents: []
  };
}

export function dashboardSummary(dashboard: DashboardState): DashboardSummary {
  return {
    applications: dashboard.applications.length,
    activeRules: dashboard.rules.filter((rule) => rule.enabled).length,
    certificates: dashboard.certificates.length,
    dnsRecords: dashboard.dnsRecords.length,
    auditEvents: dashboard.auditEvents.length
  };
}
