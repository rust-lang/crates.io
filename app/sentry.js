import * as Sentry from '@sentry/browser';
import { Ember } from '@sentry/integrations/esm/ember';

import config from './config/environment';

export function init() {
  let integrations = [];
  if (config.environment === 'production') {
    integrations.push(new Ember());
  }

  let isProd = location.hostname === 'crates.io';
  let isStaging = location.hostname === 'staging.crates.io';
  let environment = isProd ? 'production' : isStaging ? 'staging' : 'unknown';

  Sentry.init({
    environment,
    ...config.sentry,
    integrations,
  });
}
