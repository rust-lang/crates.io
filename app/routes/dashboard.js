import { A } from '@ember/array';
import { inject as service } from '@ember/service';
import RSVP from 'rsvp';

import AuthenticatedRoute from './-authenticated-route';

export default class DashboardRoute extends AuthenticatedRoute {
  @service store;

  async model() {
    let user = this.session.currentUser;

    let myCrates = this.store.query('crate', { user_id: user.get('id') });
    let myFollowing = this.store.query('crate', { following: 1 });
    let myStats = user.stats();

    return await RSVP.hash({ myCrates, myFollowing, myStats });
  }

  setupController(controller) {
    super.setupController(...arguments);

    if (!controller.isRunning) {
      controller.set('myFeed', A());
      controller.loadMoreTask.perform();
    }
  }
}
