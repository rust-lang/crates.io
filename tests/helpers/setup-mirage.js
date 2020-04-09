import setupMirage from 'ember-cli-mirage/test-support/setup-mirage';
import window, { setupWindowMock } from 'ember-window-mock';
import timekeeper from 'timekeeper';

export default function (hooks) {
  setupMirage(hooks);
  setupWindowMock(hooks);

  // To have deterministic visual tests, the seed has to be constant
  hooks.beforeEach(function () {
    timekeeper.freeze(new Date('11/20/2017 12:00'));

    this.authenticateAs = user => {
      this.server.create('mirage-session', { user });
      window.localStorage.setItem('isLoggedIn', '1');
    };
  });

  hooks.afterEach(function () {
    timekeeper.reset();
  });
}
