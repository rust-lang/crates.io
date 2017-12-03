import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import $ from 'jquery';

export default Route.extend({

    ajax: service(),

    flashMessages: service(),

    async beforeModel() {
        if (this.session.get('isLoggedIn') && this.session.get('currentUser') === null) {
            try {
                let response = await this.get('ajax').request('/api/v1/me');
                this.session.set('currentUser', this.store.push(this.store.normalize('user', response.user)));
            } catch(_) {
                this.session.logoutUser();
            } finally {
                window.currentUserDetected = true;
                $(window).trigger('currentUserDetected');
            }
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
