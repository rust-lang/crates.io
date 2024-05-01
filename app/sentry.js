import { runInDebug } from '@ember/debug';

import * as Sentry from '@sentry/browser';
import { getGlobalObject, isInstanceOf, logger } from '@sentry/utils';

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

export class Ember {
  static id = 'Ember';

  name = Ember.id;
  _Ember;

  constructor(options = {}) {
    this._Ember = options.Ember || getGlobalObject().Ember;
  }

  setupOnce(_, getCurrentHub) {
    if (!this._Ember) {
      runInDebug(() => logger.error('EmberIntegration is missing an Ember instance'));
      return;
    }

    const oldOnError = this._Ember.onerror;

    // eslint-disable-next-line unicorn/prefer-add-event-listener
    this._Ember.onerror = error => {
      if (getCurrentHub().getIntegration(Ember)) {
        getCurrentHub().captureException(error, { originalException: error });
      }

      if (typeof oldOnError === 'function') {
        oldOnError.call(this._Ember, error);
      } else if (this._Ember.testing) {
        throw error;
      }
    };

    this._Ember.RSVP.on('error', reason => {
      if (getCurrentHub().getIntegration(Ember)) {
        getCurrentHub().withScope(scope => {
          if (isInstanceOf(reason, Error)) {
            scope.setExtra('context', 'Unhandled Promise error detected');
            getCurrentHub().captureException(reason, { originalException: reason });
          } else {
            scope.setExtra('reason', reason);
            getCurrentHub().captureMessage('Unhandled Promise error detected');
          }
        });
      }
    });
  }
}
