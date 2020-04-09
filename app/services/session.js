import Service, { inject as service } from '@ember/service';

import window from 'ember-window-mock';

import ajax from '../utils/ajax';

export default class SessionService extends Service {
  @service store;
  @service router;

  savedTransition = null;
  abortedTransition = null;
  isLoggedIn = false;
  currentUser = null;
  currentUserDetected = false;
  ownedCrates = null;

  constructor() {
    super(...arguments);

    let isLoggedIn;
    try {
      isLoggedIn = window.localStorage.getItem('isLoggedIn') === '1';
    } catch (e) {
      isLoggedIn = false;
    }
    this.set('isLoggedIn', isLoggedIn);
    this.set('currentUser', null);
  }

  loginUser(user) {
    this.set('isLoggedIn', true);
    this.set('currentUser', user);
    try {
      window.localStorage.setItem('isLoggedIn', '1');
    } catch (e) {
      // ignore error
    }
  }

  logoutUser() {
    this.set('savedTransition', null);
    this.set('abortedTransition', null);
    this.set('isLoggedIn', null);
    this.set('currentUser', null);

    try {
      window.localStorage.removeItem('isLoggedIn');
    } catch (e) {
      // ignore error
    }
  }

  async loadUser() {
    if (this.isLoggedIn && !this.currentUser) {
      try {
        await this.fetchUser();
      } catch (error) {
        this.logoutUser();
      } finally {
        this.set('currentUserDetected', true);
        let transition = this.abortedTransition;
        if (transition) {
          transition.retry();
          this.set('abortedTransition', null);
        }
      }
    } else {
      this.set('currentUserDetected', true);
    }
  }

  async fetchUser() {
    let response = await ajax('/api/v1/me');
    this.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
    this.set(
      'ownedCrates',
      response.owned_crates.map(c => this.store.push(this.store.normalize('owned-crate', c))),
    );
  }

  checkCurrentUser(transition, beforeRedirect) {
    if (this.currentUser) {
      return;
    }

    // The current user is loaded asynchronously, so if we haven't actually
    // loaded the current user yet then we need to wait for it to be loaded.
    // Once we've done that we can retry the transition and start the whole
    // process over again!
    if (!this.currentUserDetected) {
      transition.abort();
      this.set('abortedTransition', transition);
    } else {
      this.set('savedTransition', transition);
      if (beforeRedirect) {
        beforeRedirect();
      }
      return this.router.transitionTo('index');
    }
  }
}
