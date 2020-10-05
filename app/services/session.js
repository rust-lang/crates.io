import { alias } from '@ember/object/computed';
import Service, { inject as service } from '@ember/service';

import { task, waitForEvent } from 'ember-concurrency';
import window from 'ember-window-mock';

import ajax from '../utils/ajax';
import * as localStorage from '../utils/local-storage';

export default class SessionService extends Service {
  @service store;
  @service notifications;
  @service router;

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
   * This task will open a popup window directed at the `github-login` route.
   * After the window has opened it will wait for the window to send a message
   * back and then evaluate whether the OAuth flow was successful.
   *
   * @see `github-authorize` route
   */
  @task(function* () {
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

    let win = window.open('/github_login', 'Authorization', windowDimensions);
    if (!win) {
      return;
    }

    let event = yield waitForEvent(window, 'message');
    if (event.origin !== window.location.origin || !event.data) {
      return;
    }

    let { data } = event.data;
    if (data && data.errors) {
      this.notifications.error(`Failed to log in: ${data.errors[0].detail}`);
      return;
    } else if (!event.data.ok) {
      this.notifications.error('Failed to log in');
      return;
    }

    this.isLoggedIn = true;

    yield this.loadUserTask.perform();

    // perform the originally saved transition, if it exists
    let transition = this.savedTransition;
    if (transition) {
      transition.retry();
    }
  })
  loginTask;

  @task(function* () {
    yield ajax(`/api/private/session`, { method: 'DELETE' });

    this.savedTransition = null;
    this.isLoggedIn = false;

    this.loadUserTask.cancelAll({ resetState: true });

    this.router.transitionTo('index');
  })
  logoutTask;

  @(task(function* () {
    if (!this.isLoggedIn) return {};

    let response;
    try {
      response = yield ajax('/api/v1/me');
    } catch (error) {
      return {};
    }

    let currentUser = this.store.push(this.store.normalize('user', response.user));
    let ownedCrates = response.owned_crates.map(c => this.store.push(this.store.normalize('owned-crate', c)));

    return { currentUser, ownedCrates };
  }).drop())
  loadUserTask;
}
