import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import ajax from 'ember-fetch/ajax';

export default Route.extend({

    async activate() {
        await ajax(`/logout`, { method: 'delete' });
        run(() => {
            this.session.logoutUser();
            this.transitionTo('index');
        });
    }
});
