import { describe, expect, test } from 'bun:test';
import { adminNavigation, adminNavigationSections, isLoginPath, resolveAdminRoute, shouldRedirectAuthenticatedUser } from './routes';

describe('admin routes', () => {
  test('resolves every navigation item to a distinct admin route', () => {
    expect(adminNavigation.map((item) => item.path)).toEqual(['/', '/applications', '/rules', '/rate-limits', '/dns', '/certificates', '/audit']);
    expect(adminNavigation.map((item) => resolveAdminRoute(item.path))).toEqual(['overview', 'applications', 'rules', 'rate-limits', 'dns', 'certificates', 'audit']);
  });

  test('groups navigation by operational areas', () => {
    expect(adminNavigationSections.map((section) => section.labelKey)).toEqual(['nav.section.account', 'nav.section.security', 'nav.section.network', 'nav.section.operations']);
    expect(adminNavigationSections.flatMap((section) => section.items.map((item) => item.route))).toEqual(adminNavigation.map((item) => item.route));
  });

  test('falls back unknown paths to overview', () => {
    expect(resolveAdminRoute('/missing')).toBe('overview');
  });

  test('redirects authenticated users away from login only', () => {
    expect(isLoginPath('/login')).toBe(true);
    expect(shouldRedirectAuthenticatedUser('/login')).toBe('/');
    expect(shouldRedirectAuthenticatedUser('/applications')).toBeNull();
  });
});
