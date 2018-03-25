import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    flashMessages: service(),
    session: service(),

    beforeModel() {
        this.get('session').loadUser();
    },

    actions: {
        didTransition() {
            this.get('flashMessages').step();
        },
    },
});
