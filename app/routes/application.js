import Ember from 'ember';
import FastBootUtils from 'cargo/mixins/fastboot-utils';

const { inject: { service } } = Ember;

export default Ember.Route.extend(FastBootUtils, {

    ajax: service(),

    flashMessages: service(),

    beforeModel() {
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null) {
            this.get('ajax').request(`${this.get('appURL')}/me`).then((response) => {
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
