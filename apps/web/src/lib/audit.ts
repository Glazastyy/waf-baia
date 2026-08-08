export type AuditEvent = {
  id: string;
  actor: string;
  action: string;
  resourceType: string;
  resourceId: string;
  result: string;
  occurredAt: string;
};

type Fetcher = typeof fetch;

export type AuditClient = {
  list: () => Promise<AuditEvent[]>;
};

export function createAuditClient(fetcher: Fetcher = fetch): AuditClient {
  return {
    list: async () => {
      const response = await fetcher('/api/audit/events', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load audit events');
      }

      return readAuditEventsList(await response.json());
    }
  };
}

function readAuditEventsList(payload: unknown): AuditEvent[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid audit events response');
  }

  return (payload as { items: unknown[] }).items.map(readAuditEvent);
}

function readAuditEvent(payload: unknown): AuditEvent {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid audit event response');
  }

  const value = payload as AuditEvent;

  if (
    typeof value.id !== 'string' ||
    typeof value.actor !== 'string' ||
    typeof value.action !== 'string' ||
    typeof value.resourceType !== 'string' ||
    typeof value.resourceId !== 'string' ||
    typeof value.result !== 'string' ||
    typeof value.occurredAt !== 'string'
  ) {
    throw new Error('Invalid audit event response');
  }

  return {
    id: value.id,
    actor: value.actor,
    action: value.action,
    resourceType: value.resourceType,
    resourceId: value.resourceId,
    result: value.result,
    occurredAt: value.occurredAt
  };
}
