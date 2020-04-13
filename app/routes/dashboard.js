import { A } from '@ember/array';
import Route from '@ember/routing/route';
import RSVP from 'rsvp';

import AuthenticatedRoute from '../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
  setupController(controller) {
    this._super(...arguments);

    if (!controller.isRunning) {
      controller.set('myFeed', A());
      controller.loadMoreTask.perform();
    }
  },

  async model() {
    let user = this.session.currentUser;

    let myCrates = this.store.query('crate', { user_id: user.get('id') });
    let myFollowing = this.store.query('crate', { following: 1 });
    let myStats = user.stats();

    return await RSVP.hash({ myCrates, myFollowing, myStats });
  },
});
