import { describe, expect, test } from 'bun:test';
import { createAuditClient } from './audit';

describe('audit client', () => {
  test('loads audit events from the API without local examples', async () => {
    const client = createAuditClient(async (input, init) => {
      expect(input).toBe('/api/audit/events');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'event-id',
            actor: 'admin',
            action: 'application.create',
            resourceType: 'application',
            resourceId: 'application-id',
            result: 'success',
            occurredAt: '123'
          }
        ]
      });
    });

    await expect(client.list()).resolves.toEqual([
      {
        id: 'event-id',
        actor: 'admin',
        action: 'application.create',
        resourceType: 'application',
        resourceId: 'application-id',
        result: 'success',
        occurredAt: '123'
      }
    ]);
  });
});
