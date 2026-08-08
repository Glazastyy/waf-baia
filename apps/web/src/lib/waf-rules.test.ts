import { describe, expect, test } from 'bun:test';
import { createWafRulesClient } from './waf-rules';

describe('waf rules client', () => {
  test('loads rules from the API without local examples', async () => {
    const client = createWafRulesClient(async (input, init) => {
      expect(input).toBe('/api/waf/rules');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'rule-id',
            name: 'Block admin paths',
            applicationId: 'app-id',
            applicationName: 'Portal',
            priority: 10,
            action: 'block',
            pathPrefix: '/admin',
            enabled: true
          }
        ]
      });
    });

    await expect(client.list()).resolves.toEqual([
      {
        id: 'rule-id',
        name: 'Block admin paths',
        applicationId: 'app-id',
        applicationName: 'Portal',
        priority: 10,
        action: 'block',
        pathPrefix: '/admin',
        enabled: true
      }
    ]);
  });

  test('creates rules with csrf protection', async () => {
    const client = createWafRulesClient(async (input, init) => {
      expect(input).toBe('/api/waf/rules');
      expect(init?.method).toBe('POST');
      expect(init?.headers).toEqual({
        accept: 'application/json',
        'content-type': 'application/json',
        'x-csrf-token': 'csrf-token'
      });
      expect(JSON.parse(String(init?.body))).toEqual({
        name: 'Block admin paths',
        applicationId: 'app-id',
        priority: 10,
        action: 'block',
        pathPrefix: '/admin'
      });

      return Response.json(
        {
          id: 'rule-id',
          name: 'Block admin paths',
          applicationId: 'app-id',
          applicationName: 'Portal',
          priority: 10,
          action: 'block',
          pathPrefix: '/admin',
          enabled: true
        },
        { status: 201 }
      );
    });

    const rule = await client.create('csrf-token', {
      name: 'Block admin paths',
      applicationId: 'app-id',
      priority: 10,
      action: 'block',
      pathPrefix: '/admin'
    });

    expect(rule.id).toBe('rule-id');
  });
});
