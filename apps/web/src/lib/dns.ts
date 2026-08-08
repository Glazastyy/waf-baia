export type DnsRecordType = 'A' | 'AAAA' | 'CNAME' | 'TXT' | 'CAA' | 'MX';

export type DnsRecord = {
  id: string;
  zoneId: string;
  zoneName: string;
  name: string;
  recordType: DnsRecordType;
  content: string;
  ttl: number;
  proxied: boolean;
};

export type CreateDnsRecordInput = {
  zoneName: string;
  name: string;
  recordType: DnsRecordType;
  content: string;
  ttl: number;
  proxied: boolean;
};

type Fetcher = typeof fetch;

export type DnsClient = {
  listRecords: () => Promise<DnsRecord[]>;
  createRecord: (csrfToken: string, input: CreateDnsRecordInput) => Promise<DnsRecord>;
};

export function createDnsClient(fetcher: Fetcher = fetch): DnsClient {
  return {
    listRecords: async () => {
      const response = await fetcher('/api/dns/records', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load DNS records');
      }

      return readDnsRecordsList(await response.json());
    },
    createRecord: async (csrfToken, input) => {
      const response = await fetcher('/api/dns/records', {
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
        throw new Error('Unable to create DNS record');
      }

      return readDnsRecord(await response.json());
    }
  };
}

function readDnsRecordsList(payload: unknown): DnsRecord[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid DNS records response');
  }

  return (payload as { items: unknown[] }).items.map(readDnsRecord);
}

function readDnsRecord(payload: unknown): DnsRecord {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid DNS record response');
  }

  const value = payload as DnsRecord;

  if (
    typeof value.id !== 'string' ||
    typeof value.zoneId !== 'string' ||
    typeof value.zoneName !== 'string' ||
    typeof value.name !== 'string' ||
    !isDnsRecordType(value.recordType) ||
    typeof value.content !== 'string' ||
    typeof value.ttl !== 'number' ||
    typeof value.proxied !== 'boolean'
  ) {
    throw new Error('Invalid DNS record response');
  }

  return {
    id: value.id,
    zoneId: value.zoneId,
    zoneName: value.zoneName,
    name: value.name,
    recordType: value.recordType,
    content: value.content,
    ttl: value.ttl,
    proxied: value.proxied
  };
}

function isDnsRecordType(value: unknown): value is DnsRecordType {
  return value === 'A' || value === 'AAAA' || value === 'CNAME' || value === 'TXT' || value === 'CAA' || value === 'MX';
}
