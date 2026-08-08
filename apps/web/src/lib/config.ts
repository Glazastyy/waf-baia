import type { MessageKey } from './i18n';

export type ModuleToggle = {
  enabled: boolean;
};

export type PlatformConfiguration = {
  modules: {
    crowdsec: ModuleToggle;
    powerdns: ModuleToggle;
  };
  integrations: {
    powerdns: {
      mode: 'integrated' | 'external';
      apiUrlConfigured: boolean;
      apiKeyConfigured: boolean;
    };
    crowdsec: {
      localApiConfigured: boolean;
      apiKeyConfigured: boolean;
    };
  };
};

export type ComponentStatus = 'configured' | 'disabled' | 'needsConfiguration';

export type ComponentConfigurationStatus = {
  id: 'crowdsec' | 'powerdns';
  name: string;
  status: ComponentStatus;
  detailKey: MessageKey;
};

type Fetcher = typeof fetch;

export type ConfigClient = {
  load: () => Promise<PlatformConfiguration>;
};

export function createConfigClient(fetcher: Fetcher = fetch): ConfigClient {
  return {
    load: async () => {
      const response = await fetcher('/api/configuration', {
        credentials: 'include',
        headers: {
          accept: 'application/json'
        }
      });

      if (!response.ok) {
        throw new Error('Unable to load platform configuration');
      }

      return readPlatformConfiguration(await response.json());
    }
  };
}

export function componentStatusesFromConfiguration(configuration: PlatformConfiguration): ComponentConfigurationStatus[] {
  return [
    {
      id: 'crowdsec',
      name: 'CrowdSec',
      status: crowdSecStatus(configuration),
      detailKey: crowdSecDetail(configuration)
    },
    {
      id: 'powerdns',
      name: 'PowerDNS',
      status: powerDnsStatus(configuration),
      detailKey: powerDnsDetail(configuration)
    }
  ];
}

function readPlatformConfiguration(payload: unknown): PlatformConfiguration {
  if (!payload || typeof payload !== 'object') {
    throw new Error('Invalid platform configuration response');
  }

  const value = payload as PlatformConfiguration;

  if (
    typeof value.modules?.crowdsec?.enabled !== 'boolean' ||
    typeof value.modules?.powerdns?.enabled !== 'boolean' ||
    !value.integrations?.powerdns ||
    !value.integrations?.crowdsec ||
    !['integrated', 'external'].includes(value.integrations.powerdns.mode) ||
    typeof value.integrations.powerdns.apiUrlConfigured !== 'boolean' ||
    typeof value.integrations.powerdns.apiKeyConfigured !== 'boolean' ||
    typeof value.integrations.crowdsec.localApiConfigured !== 'boolean' ||
    typeof value.integrations.crowdsec.apiKeyConfigured !== 'boolean'
  ) {
    throw new Error('Invalid platform configuration response');
  }

  return value;
}

function crowdSecStatus(configuration: PlatformConfiguration): ComponentStatus {
  if (!configuration.modules.crowdsec.enabled) {
    return 'disabled';
  }

  if (configuration.integrations.crowdsec.localApiConfigured && configuration.integrations.crowdsec.apiKeyConfigured) {
    return 'configured';
  }

  return 'needsConfiguration';
}

function crowdSecDetail(configuration: PlatformConfiguration): MessageKey {
  if (!configuration.modules.crowdsec.enabled) {
    return 'components.crowdsecDisabled';
  }

  if (crowdSecStatus(configuration) === 'configured') {
    return 'components.crowdsecConfigured';
  }

  return 'components.crowdsecNeedsConfiguration';
}

function powerDnsStatus(configuration: PlatformConfiguration): ComponentStatus {
  if (!configuration.modules.powerdns.enabled) {
    return 'disabled';
  }

  if (
    configuration.integrations.powerdns.mode === 'integrated' &&
    configuration.integrations.powerdns.apiUrlConfigured &&
    configuration.integrations.powerdns.apiKeyConfigured
  ) {
    return 'configured';
  }

  if (configuration.integrations.powerdns.apiUrlConfigured && configuration.integrations.powerdns.apiKeyConfigured) {
    return 'configured';
  }

  return 'needsConfiguration';
}

function powerDnsDetail(configuration: PlatformConfiguration): MessageKey {
  if (!configuration.modules.powerdns.enabled) {
    return 'components.powerdnsDisabled';
  }

  if (powerDnsStatus(configuration) !== 'configured') {
    return 'components.powerdnsNeedsConfiguration';
  }

  if (configuration.integrations.powerdns.mode === 'integrated') {
    return 'components.powerdnsIntegratedConfigured';
  }

  return 'components.powerdnsExternalConfigured';
}
