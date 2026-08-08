import { describe, expect, test } from 'bun:test';
import { createCertificatesClient } from './certificates';

describe('certificates client', () => {
  test('loads certificates from the API without local examples', async () => {
    const client = createCertificatesClient(async (input, init) => {
      expect(input).toBe('/api/certificates');
      expect(init?.credentials).toBe('include');

      return Response.json({
        items: [
          {
            id: 'certificate-id',
            applicationId: 'application-id',
            applicationName: 'Portal',
            domain: 'portal.example.com',
            issuer: 'letsencrypt',
            challengeType: 'http_01',
            status: 'pending'
          }
        ]
      });
    });

    await expect(client.list()).resolves.toEqual([
      {
        id: 'certificate-id',
        applicationId: 'application-id',
        applicationName: 'Portal',
        domain: 'portal.example.com',
        issuer: 'letsencrypt',
        challengeType: 'http_01',
        status: 'pending'
      }
    ]);
  });

  test('creates certificates with csrf protection', async () => {
    const client = createCertificatesClient(async (input, init) => {
      expect(input).toBe('/api/certificates');
      expect(init?.method).toBe('POST');
      expect(init?.headers).toEqual({
        accept: 'application/json',
        'content-type': 'application/json',
        'x-csrf-token': 'csrf-token'
      });
      expect(JSON.parse(String(init?.body))).toEqual({
        applicationId: 'application-id',
        domain: 'portal.example.com',
        issuer: 'letsencrypt',
        challengeType: 'http_01',
        status: 'pending'
      });

      return Response.json(
        {
          id: 'certificate-id',
          applicationId: 'application-id',
          applicationName: 'Portal',
          domain: 'portal.example.com',
          issuer: 'letsencrypt',
          challengeType: 'http_01',
          status: 'pending'
        },
        { status: 201 }
      );
    });

    const certificate = await client.create('csrf-token', {
      applicationId: 'application-id',
      domain: 'portal.example.com',
      issuer: 'letsencrypt',
      challengeType: 'http_01',
      status: 'pending'
    });

    expect(certificate.id).toBe('certificate-id');
  });
});
