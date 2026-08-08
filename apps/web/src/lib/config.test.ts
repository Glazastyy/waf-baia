import { describe, expect, test } from 'bun:test';
import { componentStatusesFromConfiguration, createConfigClient } from './config';

describe('configuration client', () => {
  test('loads the platform configuration from the API', async () => {
    const client = createConfigClient(async (input, init) => {
      expect(input).toBe('/api/configuration');
      expect(init?.credentials).toBe('include');

      return Response.json({
        modules: {
          crowdsec: { enabled: true },
          powerdns: { enabled: true }
        },
        integrations: {
          powerdns: {
            mode: 'integrated',
            apiUrlConfigured: true,
            apiKeyConfigured: true
          },
          crowdsec: {
            localApiConfigured: true,
            apiKeyConfigured: true
          }
        }
      });
    });

    const configuration = await client.load();

    expect(configuration.modules.crowdsec.enabled).toBe(true);
    expect(configuration.integrations.powerdns.mode).toBe('integrated');
  });

  test('marks configured integrated PowerDNS and enabled CrowdSec as configured', () => {
    const statuses = componentStatusesFromConfiguration({
      modules: {
        crowdsec: { enabled: true },
        powerdns: { enabled: true }
      },
      integrations: {
        powerdns: {
          mode: 'integrated',
          apiUrlConfigured: true,
          apiKeyConfigured: true
        },
        crowdsec: {
          localApiConfigured: true,
          apiKeyConfigured: true
        }
      }
    });

    expect(statuses).toEqual([
      { id: 'crowdsec', name: 'CrowdSec', status: 'configured', detailKey: 'components.crowdsecConfigured' },
      { id: 'powerdns', name: 'PowerDNS', status: 'configured', detailKey: 'components.powerdnsIntegratedConfigured' }
    ]);
  });
});
