import { alias } from '@ember/object/computed';
import Service, { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';
import window from 'ember-window-mock';

import ajax from '../utils/ajax';

export default class SessionService extends Service {
  @service store;
  @service router;

  savedTransition = null;
  isLoggedIn = false;

  @alias('loadUserTask.last.value.currentUser') currentUser;
  @alias('loadUserTask.last.value.ownedCrates') ownedCrates;

  constructor() {
    super(...arguments);

    let isLoggedIn;
    try {
      isLoggedIn = window.localStorage.getItem('isLoggedIn') === '1';
    } catch (e) {
      isLoggedIn = false;
    }
    this.set('isLoggedIn', isLoggedIn);
  }

  login() {
    this.set('isLoggedIn', true);
    try {
      window.localStorage.setItem('isLoggedIn', '1');
    } catch (e) {
      // ignore error
    }

    // just trigger the task, but don't wait for the result here
    this.loadUserTask.perform();

    // perform the originally saved transition, if it exists
    let transition = this.savedTransition;
    if (transition) {
      transition.retry();
    }
  }

  logoutUser() {
    this.set('savedTransition', null);
    this.set('isLoggedIn', null);

    this.loadUserTask.cancelAll({ resetState: true });

    try {
      window.localStorage.removeItem('isLoggedIn');
    } catch (e) {
      // ignore error
    }
  }

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
