export type CertificateChallengeType = 'http_01' | 'dns_01';

export type CertificateStatus = 'pending' | 'issued' | 'failed' | 'revoked';

export type Certificate = {
  id: string;
  applicationId: string | null;
  applicationName: string | null;
  domain: string;
  issuer: string;
  challengeType: CertificateChallengeType;
  status: CertificateStatus;
};

export type CreateCertificateInput = {
  applicationId: string | null;
  domain: string;
  issuer: string;
  challengeType: CertificateChallengeType;
  status: CertificateStatus;
};

type Fetcher = typeof fetch;

export type CertificatesClient = {
  list: () => Promise<Certificate[]>;
  create: (csrfToken: string, input: CreateCertificateInput) => Promise<Certificate>;
};

export function createCertificatesClient(fetcher: Fetcher = fetch): CertificatesClient {
  return {
    list: async () => {
      const response = await fetcher('/api/certificates', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load certificates');
      }

      return readCertificatesList(await response.json());
    },
    create: async (csrfToken, input) => {
      const response = await fetcher('/api/certificates', {
        method: 'POST',
        credentials: 'include',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
          'x-csrf-token': csrfToken
        },
        body: JSON.stringify(input)
      });

      if (!response.ok) {
        throw new Error('Unable to create certificate');
      }

      return readCertificate(await response.json());
    }
  };
}

function readCertificatesList(payload: unknown): Certificate[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid certificates response');
  }

  return (payload as { items: unknown[] }).items.map(readCertificate);
}

function readCertificate(payload: unknown): Certificate {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid certificate response');
  }

  const value = payload as Certificate;

  if (
    typeof value.id !== 'string' ||
    !nullableString(value.applicationId) ||
    !nullableString(value.applicationName) ||
    typeof value.domain !== 'string' ||
    typeof value.issuer !== 'string' ||
    !isCertificateChallengeType(value.challengeType) ||
    !isCertificateStatus(value.status)
  ) {
    throw new Error('Invalid certificate response');
  }

  return {
    id: value.id,
    applicationId: value.applicationId,
    applicationName: value.applicationName,
    domain: value.domain,
    issuer: value.issuer,
    challengeType: value.challengeType,
    status: value.status
  };
}

function isCertificateChallengeType(value: unknown): value is CertificateChallengeType {
  return value === 'http_01' || value === 'dns_01';
}

function isCertificateStatus(value: unknown): value is CertificateStatus {
  return value === 'pending' || value === 'issued' || value === 'failed' || value === 'revoked';
}

function nullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string';
}
