export type RateLimitAction = 'block' | 'challenge' | 'log';

export type RateLimit = {
  id: string;
  name: string;
  applicationId: string | null;
  applicationName: string | null;
  pathPrefix: string | null;
  requests: number;
  windowSeconds: number;
  action: RateLimitAction;
  enabled: boolean;
};

export type CreateRateLimitInput = {
  name: string;
  applicationId: string | null;
  pathPrefix: string | null;
  requests: number;
  windowSeconds: number;
  action: RateLimitAction;
};

type Fetcher = typeof fetch;

export type RateLimitsClient = {
  list: () => Promise<RateLimit[]>;
  create: (csrfToken: string, input: CreateRateLimitInput) => Promise<RateLimit>;
};

export function createRateLimitsClient(fetcher: Fetcher = fetch): RateLimitsClient {
  return {
    list: async () => {
      const response = await fetcher('/api/rate-limits', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load rate limits');
      }

      return readRateLimitList(await response.json());
    },
    create: async (csrfToken, input) => {
      const response = await fetcher('/api/rate-limits', {
        method: 'POST',
        credentials: 'include',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
          'x-csrf-token': csrfToken
        },
        body: JSON.stringify({
          name: input.name,
          applicationId: input.applicationId,
          pathPrefix: input.pathPrefix,
          requests: input.requests,
          windowSeconds: input.windowSeconds,
          action: input.action
        })
      });

      if (!response.ok) {
        throw new Error('Unable to create rate limit');
      }

      return readRateLimit(await response.json());
    }
  };
}

function readRateLimitList(payload: unknown): RateLimit[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid rate limits response');
  }

  return (payload as { items: unknown[] }).items.map(readRateLimit);
}

function readRateLimit(payload: unknown): RateLimit {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid rate limit response');
  }

  const value = payload as RateLimit;

  if (
    typeof value.id !== 'string' ||
    typeof value.name !== 'string' ||
    !nullableString(value.applicationId) ||
    !nullableString(value.applicationName) ||
    !nullableString(value.pathPrefix) ||
    typeof value.requests !== 'number' ||
    typeof value.windowSeconds !== 'number' ||
    !isRateLimitAction(value.action) ||
    typeof value.enabled !== 'boolean'
  ) {
    throw new Error('Invalid rate limit response');
  }

  return {
    id: value.id,
    name: value.name,
    applicationId: value.applicationId,
    applicationName: value.applicationName,
    pathPrefix: value.pathPrefix,
    requests: value.requests,
    windowSeconds: value.windowSeconds,
    action: value.action,
    enabled: value.enabled
  };
}

function isRateLimitAction(value: unknown): value is RateLimitAction {
  return value === 'block' || value === 'challenge' || value === 'log';
}

function nullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string';
}
