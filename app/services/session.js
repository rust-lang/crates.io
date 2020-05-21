import { alias } from '@ember/object/computed';
import Service, { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

import ajax from '../utils/ajax';
import * as localStorage from '../utils/local-storage';

export default class SessionService extends Service {
  @service store;
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

  login() {
    this.isLoggedIn = true;

    // just trigger the task, but don't wait for the result here
    this.loadUserTask.perform();

    // perform the originally saved transition, if it exists
    let transition = this.savedTransition;
    if (transition) {
      transition.retry();
    }
  }

  logoutUser() {
    this.savedTransition = null;
    this.isLoggedIn = false;

    this.loadUserTask.cancelAll({ resetState: true });
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
