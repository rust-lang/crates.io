import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
    ajax: service(),
    session: service(),

    async activate() {
        await this.get('ajax').delete(`/logout`);
        run(() => {
            this.get('session').logoutUser();
            this.transitionTo('index');
        });
    }
});
