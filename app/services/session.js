import Service, { inject as service } from '@ember/service';
import ajax from 'ember-fetch/ajax';

export default Service.extend({
    savedTransition: null,
    abortedTransition: null,
    isLoggedIn: false,
    currentUser: null,
    currentUserDetected: false,

    store: service(),
    router: service(),

    init() {
        this._super(...arguments);
        let isLoggedIn;
        try {
            isLoggedIn = localStorage.getItem('isLoggedIn') === '1';
        } catch (e) {
            isLoggedIn = false;
        }
        this.set('isLoggedIn', isLoggedIn);
        this.set('currentUser', null);
    },

    loginUser(user) {
        this.set('isLoggedIn', true);
        this.set('currentUser', user);
        try {
            localStorage.setItem('isLoggedIn', '1');
        } catch (e) {
            // ignore error
        }
    },

    logoutUser() {
        this.set('savedTransition', null);
        this.set('abortedTransition', null);
        this.set('isLoggedIn', null);
        this.set('currentUser', null);

        try {
            localStorage.removeItem('isLoggedIn');
        } catch (e) {
            // ignore error
        }
    },

    loadUser() {
        if (this.isLoggedIn && !this.currentUser) {
            this.fetchUser()
                .catch(() => this.logoutUser())
                .finally(() => {
                    this.set('currentUserDetected', true);
                    let transition = this.abortedTransition;
                    if (transition) {
                        transition.retry();
                        this.set('abortedTransition', null);
                    }
                });
        } else {
            this.set('currentUserDetected', true);
        }
    },

    fetchUser() {
        return ajax('/api/v1/me').then(response => {
            this.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
        });
    },

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
    },
});
