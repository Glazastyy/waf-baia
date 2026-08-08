import type { MessageKey } from './i18n';

export type AdminRoute = 'overview' | 'applications' | 'rules' | 'dns' | 'audit';

export type AdminNavigationItem = {
  route: AdminRoute;
  path: string;
  labelKey: MessageKey;
  icon: string;
};

export const adminNavigation: AdminNavigationItem[] = [
  { route: 'overview', path: '/', labelKey: 'nav.overview', icon: 'bi-speedometer2' },
  { route: 'applications', path: '/applications', labelKey: 'nav.applications', icon: 'bi-window-stack' },
  { route: 'rules', path: '/rules', labelKey: 'nav.rules', icon: 'bi-shield-check' },
  { route: 'dns', path: '/dns', labelKey: 'nav.dns', icon: 'bi-diagram-3' },
  { route: 'audit', path: '/audit', labelKey: 'nav.audit', icon: 'bi-clock-history' }
];

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
