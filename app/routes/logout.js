import Route from '@ember/routing/route';
import { run } from '@ember/runloop';
import { inject as service } from '@ember/service';

import ajax from '../utils/ajax';

export default Route.extend({
  session: service(),

  async activate() {
    await ajax(`/api/private/session`, { method: 'DELETE' });
    run(() => {
      this.session.logoutUser();
      this.transitionTo('index');
    });
  },
});
