import Ember from 'ember';

const { inject: { service } } = Ember;

export default Ember.Route.extend({

    ajax: service(),

    flashMessages: service(),

    beforeModel() {
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null) {
            this.get('ajax').request('/me').then((response) => {
                this.session.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
            }).catch(() => this.session.logoutUser()).finally(() => {
                window.currentUserDetected = true;
                Ember.$(window).trigger('currentUserDetected');
            });
        } else {
            window.currentUserDetected = true;
        }
    },

    actions: {
        didTransition() {
            this.get('flashMessages').step();
        },
    },
});
