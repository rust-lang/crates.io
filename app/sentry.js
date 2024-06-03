import * as Sentry from '@sentry/ember';

import config from './config/environment';

export function init() {
  let isProd = location.hostname === 'crates.io';
  let isStaging = location.hostname === 'staging.crates.io';
  let environment = isProd ? 'production' : isStaging ? 'staging' : 'unknown';

  Sentry.init({
    environment,
    ...config.sentry,

    allowUrls: ['crates.io', 'crates-io.herokuapp.com', 'staging.crates-io', 'staging-crates-io.herokuapp.com'],

    beforeSend(event, hint) {
      let error = hint?.originalException;

      // Ignoring these errors due to https://github.com/emberjs/ember.js/issues/12505
      // and https://github.com/emberjs/ember.js/issues/18416
      if (error && error.name === 'TransitionAborted') {
        return null;
      }

      return event;
    },
  });
}
