import { setupMirage } from 'ember-cli-mirage/test-support';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupFakeTimers } from './fake-timers';

export default function (hooks) {
  setupMirage(hooks);
  setupWindowMock(hooks);
  setupFakeTimers(hooks, '2017-11-20T12:00:00');

  // To have deterministic visual tests, the seed has to be constant
  hooks.beforeEach(function () {
    this.authenticateAs = user => {
      this.server.create('mirage-session', { user });
      window.localStorage.setItem('isLoggedIn', '1');
    };
  });
}
