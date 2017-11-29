import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),

    beforeModel() {
        this.session.loadUser();
    },

    actions: {
        didTransition() {
            this.get('flashMessages').step();
        },
    },
});
