import { describe, expect, test } from 'bun:test';
import { createRateLimitsClient } from './rate-limits';

describe('rate limits client', () => {
  test('loads rate limits from the API without local examples', async () => {
    const client = createRateLimitsClient(async (input, init) => {
      expect(input).toBe('/api/rate-limits');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'limit_1',
            name: 'Login burst',
            applicationId: 'app_1',
            applicationName: 'Portal',
            pathPrefix: '/login',
            requests: 20,
            windowSeconds: 60,
            action: 'block',
            enabled: true
          }
        ]
      });
    });

    await expect(client.list()).resolves.toEqual([
      {
        id: 'limit_1',
        name: 'Login burst',
        applicationId: 'app_1',
        applicationName: 'Portal',
        pathPrefix: '/login',
        requests: 20,
        windowSeconds: 60,
        action: 'block',
        enabled: true
      }
    ]);
  });

  test('creates rate limits with csrf protection', async () => {
    const client = createRateLimitsClient(async (input, init) => {
      expect(input).toBe('/api/rate-limits');
      expect(init?.method).toBe('POST');
      expect((init?.headers as Record<string, string>)['x-csrf-token']).toBe('csrf-token');
      expect(JSON.parse(init?.body as string)).toEqual({
        name: 'Login burst',
        applicationId: null,
        pathPrefix: '/login',
        requests: 20,
        windowSeconds: 60,
        action: 'block'
      });

      return Response.json(
        {
          id: 'limit_1',
          name: 'Login burst',
          applicationId: null,
          applicationName: null,
          pathPrefix: '/login',
          requests: 20,
          windowSeconds: 60,
          action: 'block',
          enabled: true
        },
        { status: 201 }
      );
    });

    await expect(
      client.create('csrf-token', {
        name: 'Login burst',
        applicationId: null,
        pathPrefix: '/login',
        requests: 20,
        windowSeconds: 60,
        action: 'block'
      })
    ).resolves.toMatchObject({ id: 'limit_1', enabled: true });
  });
});
