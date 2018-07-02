import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import ajax from 'ember-fetch/ajax';

export default Route.extend({
    session: service(),

    async activate() {
        await ajax(`/logout`, { method: 'DELETE' });
        run(() => {
            this.session.logoutUser();
            this.transitionTo('index');
        });
    },
});
