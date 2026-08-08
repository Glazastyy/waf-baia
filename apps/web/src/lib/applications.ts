export type ApplicationUpstream = {
  id: string;
  dial: string;
  weight: number;
  enabled: boolean;
};

export type Application = {
  id: string;
  name: string;
  hostname: string;
  enabled: boolean;
  upstreams: ApplicationUpstream[];
};

export type CreateApplicationInput = {
  name: string;
  hostname: string;
  upstreamDial: string;
};

type Fetcher = typeof fetch;

export type ApplicationsClient = {
  list: () => Promise<Application[]>;
  create: (csrfToken: string, input: CreateApplicationInput) => Promise<Application>;
};

export function createApplicationsClient(fetcher: Fetcher = fetch): ApplicationsClient {
  return {
    list: async () => {
      const response = await fetcher('/api/applications', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load applications');
      }

      return readApplicationsList(await response.json());
    },
    create: async (csrfToken, input) => {
      const response = await fetcher('/api/applications', {
        method: 'POST',
        credentials: 'include',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
          'x-csrf-token': csrfToken
        },
        body: JSON.stringify({
          name: input.name,
          hostname: input.hostname,
          upstreams: [{ dial: input.upstreamDial }]
        })
      });

      if (!response.ok) {
        throw new Error('Unable to create application');
      }

      return readApplication(await response.json());
    }
  };
}

function readApplicationsList(payload: unknown): Application[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid applications response');
  }

  return (payload as { items: unknown[] }).items.map(readApplication);
}

function readApplication(payload: unknown): Application {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid application response');
  }

  const value = payload as Application;

  if (
    typeof value.id !== 'string' ||
    typeof value.name !== 'string' ||
    typeof value.hostname !== 'string' ||
    typeof value.enabled !== 'boolean' ||
    !Array.isArray(value.upstreams)
  ) {
    throw new Error('Invalid application response');
  }

  return {
    id: value.id,
    name: value.name,
    hostname: value.hostname,
    enabled: value.enabled,
    upstreams: value.upstreams.map(readUpstream)
  };
}

function readUpstream(payload: unknown): ApplicationUpstream {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid upstream response');
  }

  const value = payload as ApplicationUpstream;

  if (
    typeof value.id !== 'string' ||
    typeof value.dial !== 'string' ||
    typeof value.weight !== 'number' ||
    typeof value.enabled !== 'boolean'
  ) {
    throw new Error('Invalid upstream response');
  }

  return {
    id: value.id,
    dial: value.dial,
    weight: value.weight,
    enabled: value.enabled
  };
}
