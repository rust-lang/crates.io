import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
    beforeModel() {
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null) {
            ajax('/me').then((response) => {
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
            this.controllerFor('application').stepFlash();
        },
    },
});
