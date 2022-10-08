import Service, { inject as service } from '@ember/service';

import { dropTask, race, rawTimeout, task, waitForEvent } from 'ember-concurrency';
import window from 'ember-window-mock';
import { alias } from 'macro-decorators';

import ajax from '../utils/ajax';
import * as localStorage from '../utils/local-storage';

export default class SessionService extends Service {
  @service store;
  @service notifications;
  @service router;
  @service sentry;

  savedTransition = null;

  @alias('loadUserTask.last.value.currentUser') currentUser;
  @alias('loadUserTask.last.value.ownedCrates') ownedCrates;

  get isLoggedIn() {
    return localStorage.getItem('isLoggedIn') === '1';
  }

  set isLoggedIn(value) {
    if (value) {
      localStorage.setItem('isLoggedIn', '1');
    } else {
      localStorage.removeItem('isLoggedIn');
    }
  }

  /**
   * This task will open a popup window, query the `/api/private/session/begin` API
   * endpoint and then navigate the popup window to the received URL.
   *
   * Example URL:
   * https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg
   *
   * Once the user has allowed the OAuth flow access the page will redirect him
   * to the `github-authorize` route of this application.
   *
   * The task will then wait for the window to send a message back and evaluate
   * whether the OAuth flow was successful.
   *
   * @see https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access
   * @see `github-authorize` route
   */
  loginTask = task(async () => {
    let windowDimensions = [
      'width=1000',
      'height=450',
      'toolbar=0',
      'scrollbars=1',
      'status=1',
      'resizable=1',
      'location=1',
      'menuBar=0',
    ].join(',');

    let win = window.open('', '_blank', windowDimensions);
    if (!win) {
      return;
    }

    win.document.write('<html><head></head><body>Please wait while we redirect you…</body></html>');
    win.document.close();

    // we can't call `window.open()` with this URL directly, because it might trigger
    // the popup window prevention mechanism of the browser, since the async opening
    // can not be associated with the original user click event
    let { url } = await ajax(`/api/private/session/begin`);
    win.location = url;

    let event = await race([this.windowEventWatcherTask.perform(), this.windowCloseWatcherTask.perform(win)]);
    if (event.closed) {
      this.notifications.warning('Login was canceled because the popup window was closed.');
      return;
    }

    win.close();

    let { code, state } = event;

    let response = await fetch(`/api/private/session/authorize?code=${code}&state=${state}`);
    if (!response.ok) {
      let json = await response.json();

      if (json && json.errors) {
        this.notifications.error(`Failed to log in: ${json.errors[0].detail}`);
      } else {
        this.notifications.error('Failed to log in');
      }
      return;
    }

    this.isLoggedIn = true;

    await this.loadUserTask.perform();

    // perform the originally saved transition, if it exists
    let transition = this.savedTransition;
    if (transition) {
      transition.retry();
    }
  });

  windowEventWatcherTask = task(async () => {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      let event = await waitForEvent(window, 'message');
      if (event.origin !== window.location.origin || !event.data) {
        continue;
      }

      let { code, state } = event.data;
      if (!code || !state) {
        continue;
      }

      return { code, state };
    }
  });

  windowCloseWatcherTask = task(async window => {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      if (window.closed) {
        return { closed: true };
      }
      await rawTimeout(10);
    }
  });

  logoutTask = task(async () => {
    await ajax(`/api/private/session`, { method: 'DELETE' });

    this.isLoggedIn = false;

    // We perform a proper page navigation here instead of an in-app transition to ensure
    // that the Ember Data store and any other in-memory data is cleared on logout.
    window.location.assign('/');
  });

  loadUserTask = dropTask(async () => {
    if (!this.isLoggedIn) return {};

    let response;
    try {
      response = await ajax('/api/v1/me');
    } catch {
      return {};
    }

    let currentUser = this.store.push(this.store.normalize('user', response.user));
    let ownedCrates = response.owned_crates.map(c => this.store.push(this.store.normalize('owned-crate', c)));

    let { id } = currentUser;
    this.sentry.setUser({ id });

    return { currentUser, ownedCrates };
  });
}
