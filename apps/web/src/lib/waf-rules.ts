export type WafRuleAction = 'allow' | 'block' | 'challenge' | 'rate_limit' | 'log';

export type WafRule = {
  id: string;
  name: string;
  applicationId: string | null;
  applicationName: string | null;
  priority: number;
  action: WafRuleAction;
  pathPrefix: string | null;
  enabled: boolean;
};

export type CreateWafRuleInput = {
  name: string;
  applicationId: string | null;
  priority: number;
  action: WafRuleAction;
  pathPrefix: string | null;
};

type Fetcher = typeof fetch;

export type WafRulesClient = {
  list: () => Promise<WafRule[]>;
  create: (csrfToken: string, input: CreateWafRuleInput) => Promise<WafRule>;
};

export function createWafRulesClient(fetcher: Fetcher = fetch): WafRulesClient {
  return {
    list: async () => {
      const response = await fetcher('/api/waf/rules', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load WAF rules');
      }

      return readWafRulesList(await response.json());
    },
    create: async (csrfToken, input) => {
      const response = await fetcher('/api/waf/rules', {
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
          priority: input.priority,
          action: input.action,
          pathPrefix: input.pathPrefix
        })
      });

      if (!response.ok) {
        throw new Error('Unable to create WAF rule');
      }

      return readWafRule(await response.json());
    }
  };
}

function readWafRulesList(payload: unknown): WafRule[] {
  if (!payload || typeof payload !== 'object' || !Array.isArray((payload as { items?: unknown }).items)) {
    throw new Error('Invalid WAF rules response');
  }

  return (payload as { items: unknown[] }).items.map(readWafRule);
}

function readWafRule(payload: unknown): WafRule {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid WAF rule response');
  }

  const value = payload as WafRule;

  if (
    typeof value.id !== 'string' ||
    typeof value.name !== 'string' ||
    typeof value.priority !== 'number' ||
    !isWafRuleAction(value.action) ||
    typeof value.enabled !== 'boolean' ||
    !nullableString(value.applicationId) ||
    !nullableString(value.applicationName) ||
    !nullableString(value.pathPrefix)
  ) {
    throw new Error('Invalid WAF rule response');
  }

  return {
    id: value.id,
    name: value.name,
    applicationId: value.applicationId,
    applicationName: value.applicationName,
    priority: value.priority,
    action: value.action,
    pathPrefix: value.pathPrefix,
    enabled: value.enabled
  };
}

function isWafRuleAction(value: unknown): value is WafRuleAction {
  return value === 'allow' || value === 'block' || value === 'challenge' || value === 'rate_limit' || value === 'log';
}

function nullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string';
}
