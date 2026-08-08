import type { MessageKey } from './i18n';

export type AdminRoute = 'overview' | 'applications' | 'rules' | 'rate-limits' | 'dns' | 'certificates' | 'audit';

export type AdminNavigationItem = {
  route: AdminRoute;
  path: string;
  labelKey: MessageKey;
  icon: string;
};

export type AdminNavigationSection = {
  labelKey: MessageKey;
  items: AdminNavigationItem[];
};

export const adminNavigationSections: AdminNavigationSection[] = [
  {
    labelKey: 'nav.section.account',
    items: [
      { route: 'overview', path: '/', labelKey: 'nav.overview', icon: 'bi-speedometer2' },
      { route: 'applications', path: '/applications', labelKey: 'nav.applications', icon: 'bi-window-stack' }
    ]
  },
  {
    labelKey: 'nav.section.security',
    items: [
      { route: 'rules', path: '/rules', labelKey: 'nav.rules', icon: 'bi-shield-check' },
      { route: 'rate-limits', path: '/rate-limits', labelKey: 'nav.rateLimits', icon: 'bi-stopwatch' }
    ]
  },
  {
    labelKey: 'nav.section.network',
    items: [
      { route: 'dns', path: '/dns', labelKey: 'nav.dns', icon: 'bi-diagram-3' },
      { route: 'certificates', path: '/certificates', labelKey: 'nav.certificates', icon: 'bi-patch-check' }
    ]
  },
  {
    labelKey: 'nav.section.operations',
    items: [
      { route: 'audit', path: '/audit', labelKey: 'nav.audit', icon: 'bi-clock-history' }
    ]
  }
];

export const adminNavigation = adminNavigationSections.flatMap((section) => section.items);

export function resolveAdminRoute(pathname: string): AdminRoute {
  return adminNavigation.find((item) => item.path === pathname)?.route ?? 'overview';
}

export function pathForRoute(route: AdminRoute): string {
  return adminNavigation.find((item) => item.route === route)?.path ?? '/';
}

export function isLoginPath(pathname: string): boolean {
  return pathname === '/login';
}

export function shouldRedirectAuthenticatedUser(pathname: string): string | null {
  return isLoginPath(pathname) ? '/' : null;
}
