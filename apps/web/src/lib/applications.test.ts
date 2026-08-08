import { describe, expect, test } from 'bun:test';
import { createApplicationsClient } from './applications';

describe('applications client', () => {
  test('loads applications from the API without inventing local rows', async () => {
    const client = createApplicationsClient(async (input, init) => {
      expect(input).toBe('/api/applications');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'app-id',
            name: 'Portal',
            hostname: 'portal.example.com',
            enabled: true,
            upstreams: [{ id: 'upstream-id', dial: '10.0.0.20:8080', weight: 1, enabled: true }]
          }
        ]
      });
    });

    await expect(client.list()).resolves.toEqual([
      {
        id: 'app-id',
        name: 'Portal',
        hostname: 'portal.example.com',
        enabled: true,
        upstreams: [{ id: 'upstream-id', dial: '10.0.0.20:8080', weight: 1, enabled: true }]
      }
    ]);
  });

  test('creates applications with csrf protection', async () => {
    const client = createApplicationsClient(async (input, init) => {
      expect(input).toBe('/api/applications');
      expect(init?.method).toBe('POST');
      expect(init?.headers).toEqual({
        accept: 'application/json',
        'content-type': 'application/json',
        'x-csrf-token': 'csrf-token'
      });
      expect(JSON.parse(String(init?.body))).toEqual({
        name: 'Portal',
        hostname: 'portal.example.com',
        upstreams: [{ dial: '10.0.0.20:8080' }]
      });

      return Response.json(
        {
          id: 'app-id',
          name: 'Portal',
          hostname: 'portal.example.com',
          enabled: true,
          upstreams: [{ id: 'upstream-id', dial: '10.0.0.20:8080', weight: 1, enabled: true }]
        },
        { status: 201 }
      );
    });

    const application = await client.create('csrf-token', {
      name: 'Portal',
      hostname: 'portal.example.com',
      upstreamDial: '10.0.0.20:8080'
    });

    expect(application.id).toBe('app-id');
  });
});
