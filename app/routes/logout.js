import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import ajax from 'ember-fetch/ajax';
import { inject as service } from '@ember/service';

export default Route.extend({
    session: service(),

    async activate() {
        await ajax(`/logout`, { method: 'delete' });
        run(() => {
            this.get('session').logoutUser();
            this.transitionTo('index');
        });
    }
});
