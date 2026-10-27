import commonEn from '../locales/en-US/common.json' with { type: 'json' };
import navigationEn from '../locales/en-US/navigation.json' with { type: 'json' };
import healthEn from '../locales/en-US/health.json' with { type: 'json' };
import homeEn from '../locales/en-US/home.json' with { type: 'json' };
import agentsEn from '../locales/en-US/agents.json' with { type: 'json' };
import modelsEn from '../locales/en-US/models.json' with { type: 'json' };
import providersEn from '../locales/en-US/providers.json' with { type: 'json' };
import extensionsEn from '../locales/en-US/extensions.json' with { type: 'json' };
import systemEn from '../locales/en-US/system.json' with { type: 'json' };
import settingsEn from '../locales/en-US/settings.json' with { type: 'json' };
import diagnosticsEn from '../locales/en-US/diagnostics.json' with { type: 'json' };
import commonZh from '../locales/zh-CN/common.json' with { type: 'json' };
import navigationZh from '../locales/zh-CN/navigation.json' with { type: 'json' };
import healthZh from '../locales/zh-CN/health.json' with { type: 'json' };
import homeZh from '../locales/zh-CN/home.json' with { type: 'json' };
import agentsZh from '../locales/zh-CN/agents.json' with { type: 'json' };
import modelsZh from '../locales/zh-CN/models.json' with { type: 'json' };
import providersZh from '../locales/zh-CN/providers.json' with { type: 'json' };
import extensionsZh from '../locales/zh-CN/extensions.json' with { type: 'json' };
import systemZh from '../locales/zh-CN/system.json' with { type: 'json' };
import settingsZh from '../locales/zh-CN/settings.json' with { type: 'json' };
import diagnosticsZh from '../locales/zh-CN/diagnostics.json' with { type: 'json' };
import checksEn from '../locales/en-US/checks.json' with { type: 'json' };
import findingsEn from '../locales/en-US/findings.json' with { type: 'json' };
import actionsEn from '../locales/en-US/actions.json' with { type: 'json' };
import errorsEn from '../locales/en-US/errors.json' with { type: 'json' };
import capabilityEn from '../locales/en-US/capability.json' with { type: 'json' };
import checksZh from '../locales/zh-CN/checks.json' with { type: 'json' };
import findingsZh from '../locales/zh-CN/findings.json' with { type: 'json' };
import actionsZh from '../locales/zh-CN/actions.json' with { type: 'json' };
import errorsZh from '../locales/zh-CN/errors.json' with { type: 'json' };
import capabilityZh from '../locales/zh-CN/capability.json' with { type: 'json' };

export const resources = {
  'en-US': { common: commonEn, navigation: navigationEn, health: healthEn, home: homeEn, agents: agentsEn, models: modelsEn, providers: providersEn, extensions: extensionsEn, system: systemEn, settings: settingsEn, diagnostics: diagnosticsEn, checks: checksEn, findings: findingsEn, actions: actionsEn, errors: errorsEn, capability: capabilityEn },
  'zh-CN': { common: commonZh, navigation: navigationZh, health: healthZh, home: homeZh, agents: agentsZh, models: modelsZh, providers: providersZh, extensions: extensionsZh, system: systemZh, settings: settingsZh, diagnostics: diagnosticsZh, checks: checksZh, findings: findingsZh, actions: actionsZh, errors: errorsZh, capability: capabilityZh },
};
