import { db, handlers } from '@crates-io/msw';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';
import { http, passthrough } from 'msw';
import { setupWorker } from 'msw/browser';

import { setupFakeTimers } from './fake-timers';

export default function (hooks) {
  setupWindowMock(hooks);
  setupFakeTimers(hooks, '2017-11-20T12:00:00');

  let worker = setupWorker(
    ...handlers,
    http.get('/assets/*', passthrough),
    http.all(/.*\/percy\/.*/, passthrough),
    http.get('https://:avatars.githubusercontent.com/u/:id', passthrough),
  );

  hooks.before(() => worker.start({ quiet: true, onUnhandledRequest: 'error' }));
  hooks.afterEach(() => worker.resetHandlers());
  hooks.afterEach(() => db.reset());
  hooks.after(() => worker.stop());

  hooks.beforeEach(function () {
    this.worker = worker;
    this.db = db;

    this.authenticateAs = user => {
      db.mswSession.create({ user });
      window.localStorage.setItem('isLoggedIn', '1');
    };
  });
}
