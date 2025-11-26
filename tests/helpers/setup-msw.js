import { db, handlers } from '@crates-io/msw';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';
import { http, passthrough } from 'msw';
import { setupWorker } from 'msw/browser';

import { setupFakeTimers } from './fake-timers';

const worker = setupWorker(
  ...handlers,
  http.get('/assets/*', passthrough),
  http.all(/.*\/percy\/.*/, passthrough),
  http.get('https://:avatars.githubusercontent.com/u/:id', passthrough),
);

export function registerQUnitCallbacks(QUnit) {
  QUnit.begin(() => worker.start({ quiet: true, onUnhandledRequest: 'error' }));
  QUnit.testDone(() => worker.resetHandlers());
  QUnit.testDone(() => db.reset());
  QUnit.done(() => worker.stop());
}

export default function (hooks) {
  setupWindowMock(hooks);
  setupFakeTimers(hooks, '2017-11-20T12:00:00');

  hooks.beforeEach(function () {
    this.worker = worker;
    this.db = db;

    this.authenticateAs = async user => {
      await db.mswSession.create({ user });
      window.localStorage.setItem('isLoggedIn', '1');
    };
  });
}
