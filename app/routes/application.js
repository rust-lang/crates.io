import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import $ from 'jquery';

export default Route.extend({

    ajax: service(),

    flashMessages: service(),

    beforeModel() {
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null) {
            this.get('ajax').request('/api/v1/me').then((response) => {
                this.session.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
            }).catch(() => this.session.logoutUser()).finally(() => {
                window.currentUserDetected = true;
                $(window).trigger('currentUserDetected');
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
