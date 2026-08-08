import { describe, expect, test } from 'bun:test';
import { createAuthClient } from './auth';

describe('auth client', () => {
  test('reports anonymous session without inventing local authentication state', async () => {
    const calls: RequestInit[] = [];
    const client = createAuthClient(async (_input, init) => {
      calls.push(init ?? {});
      return new Response(JSON.stringify({ error: 'Authentication required' }), { status: 401 });
    });

    const session = await client.session();

    expect(session.authenticated).toBe(false);
    expect(calls[0]?.credentials).toBe('include');
  });

  test('logs in through the API and keeps the csrf token in memory', async () => {
    const client = createAuthClient(async (input, init) => {
      expect(input).toBe('/api/auth/login');
      expect(init?.credentials).toBe('include');
      expect(init?.method).toBe('POST');
      return new Response(
        JSON.stringify({
          csrfToken: 'csrf-token-value-with-enough-random-length',
          user: { username: 'admin', passwordChangeRequired: true }
        }),
        { status: 200, headers: { 'content-type': 'application/json' } }
      );
    });

    const session = await client.login('admin', 'secret-password');

    expect(session.authenticated).toBe(true);
    expect(session.csrfToken).toBe('csrf-token-value-with-enough-random-length');
    expect(session.user?.username).toBe('admin');
  });

  test('sends csrf token for logout and clears the local session', async () => {
    const requests: RequestInit[] = [];
    const client = createAuthClient(async (_input, init) => {
      requests.push(init ?? {});
      if (init?.method === 'POST' && !Object.hasOwn(init.headers ?? {}, 'x-csrf-token')) {
        return new Response(
          JSON.stringify({
            csrfToken: 'csrf-token-value-with-enough-random-length',
            user: { username: 'admin', passwordChangeRequired: false }
          }),
          { status: 200 }
        );
      }

      return new Response(null, { status: 204 });
    });

    await client.login('admin', 'secret-password');
    const session = await client.logout();

    expect(requests[1]?.headers).toEqual({ 'x-csrf-token': 'csrf-token-value-with-enough-random-length' });
    expect(session.authenticated).toBe(false);
  });
});
