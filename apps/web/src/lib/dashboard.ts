import type { Application } from './applications';

export type { Application } from './applications';

export type WafRule = {
  id: string;
  name: string;
  applicationName: string | null;
  action: string;
  enabled: boolean;
};

export type Certificate = {
  id: string;
  domain: string;
  issuer: string;
  status: string;
  renewal: string | null;
};

export type DnsRecord = {
  id: string;
  type: string;
  name: string;
  value: string;
  proxied: boolean;
};

export type AuditEvent = {
  id: string;
  actor: string;
  action: string;
  resource: string;
  createdAt: string;
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
