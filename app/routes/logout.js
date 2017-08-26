import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({

    ajax: service(),

    activate() {
        this.get('ajax').delete(`/logout`).then(() => {
            run(() => {
                this.session.logoutUser();
                this.transitionTo('index');
            });
        });
    }
});
