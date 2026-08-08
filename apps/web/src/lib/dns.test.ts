import { describe, expect, test } from 'bun:test';
import { createDnsClient } from './dns';

describe('dns client', () => {
  test('loads dns records from the API without local examples', async () => {
    const client = createDnsClient(async (input, init) => {
      expect(input).toBe('/api/dns/records');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'record-id',
            zoneId: 'zone-id',
            zoneName: 'example.com',
            name: 'portal.example.com',
            recordType: 'A',
            content: '10.0.0.20',
            ttl: 300,
            proxied: false
          }
        ]
      });
    });

    await expect(client.listRecords()).resolves.toEqual([
      {
        id: 'record-id',
        zoneId: 'zone-id',
        zoneName: 'example.com',
        name: 'portal.example.com',
        recordType: 'A',
        content: '10.0.0.20',
        ttl: 300,
        proxied: false
      }
    ]);
  });

  test('creates dns records with csrf protection', async () => {
    const client = createDnsClient(async (input, init) => {
      expect(input).toBe('/api/dns/records');
      expect(init?.method).toBe('POST');
      expect(init?.headers).toEqual({
        accept: 'application/json',
        'content-type': 'application/json',
        'x-csrf-token': 'csrf-token'
      });
      expect(JSON.parse(String(init?.body))).toEqual({
        zoneName: 'example.com',
        name: 'portal.example.com',
        recordType: 'A',
        content: '10.0.0.20',
        ttl: 300,
        proxied: false
      });

      return Response.json(
        {
          id: 'record-id',
          zoneId: 'zone-id',
          zoneName: 'example.com',
          name: 'portal.example.com',
          recordType: 'A',
          content: '10.0.0.20',
          ttl: 300,
          proxied: false
        },
        { status: 201 }
      );
    });

    const record = await client.createRecord('csrf-token', {
      zoneName: 'example.com',
      name: 'portal.example.com',
      recordType: 'A',
      content: '10.0.0.20',
      ttl: 300,
      proxied: false
    });

    expect(record.id).toBe('record-id');
  });
});
