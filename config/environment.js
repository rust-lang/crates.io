'use strict';

module.exports = function (environment) {
  let ENV = {
    modulePrefix: 'cargo',
    environment,
    rootURL: '/',
    locationType: 'auto',
    historySupportMiddleware: true,
    EmberENV: {
      FEATURES: {
        // Here you can enable experimental features on an ember canary build
        // e.g. 'with-controller': true
      },
      EXTEND_PROTOTYPES: {
        // Prevent Ember Data from overriding Date.parse.
        Date: false,
      },
    },

    APP: {
      // Here you can pass flags/options to your application instance
      // when it is created
    },

    fastboot: {
      hostWhitelist: ['crates.io', /^localhost:\d+$/, /\.herokuapp\.com$/],
    },

    'ember-cli-notifications': {
      autoClear: true,
      clearDuration: 10_000,
    },
  };

  if (environment === 'development') {
    // ENV.APP.LOG_RESOLVER = true;
    // ENV.APP.LOG_ACTIVE_GENERATION = true;
    // ENV.APP.LOG_TRANSITIONS = true;
    // ENV.APP.LOG_TRANSITIONS_INTERNAL = true;
    // ENV.APP.LOG_VIEW_LOOKUPS = true;
    ENV['ember-a11y-testing'] = {
      componentOptions: {
        turnAuditOff: true,
      },
    };
  }

  if (environment === 'test') {
    // Testem prefers this...
    ENV.locationType = 'none';

    // keep test console output quieter
    ENV.APP.LOG_ACTIVE_GENERATION = false;
    ENV.APP.LOG_VIEW_LOOKUPS = false;

    ENV.APP.rootElement = '#ember-testing';
    ENV.APP.autoboot = false;

    // disable auto clearing so that we can manually clear the queue if needed
    ENV['ember-cli-notifications'].autoClear = false;
  }

  if (environment === 'production') {
    // here you can enable a production-specific feature
    delete ENV['ember-cli-mirage'];

    ENV.sentry = {
      dsn: process.env.SENTRY_DSN_WEB,
    };

    // Heroku Git Hash support
    if (process.env.SOURCE_VERSION) {
      let hash = process.env.SOURCE_VERSION.slice(0, 7);
      ENV['ember-cli-app-version'] = { version: hash };
      ENV.sentry.release = hash;
    }
  }

  return ENV;
};
