import Service from '@ember/service';

import * as Sentry from '@sentry/ember';

export default class SentryService extends Service {
  captureException(error, captureContext) {
    Sentry.captureException(error, captureContext);
  }

  getCurrentScope() {
    return Sentry.getCurrentScope();
  }

  setUser(user) {
    Sentry.setUser(user);
  }
}
