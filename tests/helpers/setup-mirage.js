import { settled } from '@ember/test-helpers';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import makeServer from '../mirage/config';
import { setupFakeTimers } from './fake-timers';

export default function (hooks) {
  setupWindowMock(hooks);
  setupFakeTimers(hooks, '2017-11-20T12:00:00');

  // To have deterministic visual tests, the seed has to be constant
  hooks.beforeEach(function () {
    this.server = makeServer({
      environment: 'test',
    });

    if (location.search.includes('mirageLogging')) {
      this.server.logging = true;
    }

    this.authenticateAs = user => {
      this.server.create('mirage-session', { user });
      window.localStorage.setItem('isLoggedIn', '1');
    };
  });

  hooks.afterEach(async function () {
    await settled();

    if (this.server) {
      this.server.shutdown();
      delete this.server;
    }
  });
}
