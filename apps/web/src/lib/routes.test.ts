import { describe, expect, test } from 'bun:test';
import { adminNavigation, isLoginPath, resolveAdminRoute, shouldRedirectAuthenticatedUser } from './routes';

describe('admin routes', () => {
  test('resolves every navigation item to a distinct admin route', () => {
    expect(adminNavigation.map((item) => item.path)).toEqual(['/', '/applications', '/rules', '/rate-limits', '/dns', '/audit']);
    expect(adminNavigation.map((item) => resolveAdminRoute(item.path))).toEqual(['overview', 'applications', 'rules', 'rate-limits', 'dns', 'audit']);
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
