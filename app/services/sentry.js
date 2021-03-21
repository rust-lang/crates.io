import Service from '@ember/service';

import * as Sentry from '@sentry/browser';

export default class SentryService extends Service {
  captureException(error, captureContext) {
    Sentry.captureException(error, captureContext);
  }

  configureScope(callback) {
    Sentry.configureScope(callback);
  }

  setUser(user) {
    Sentry.setUser(user);
  }
}
