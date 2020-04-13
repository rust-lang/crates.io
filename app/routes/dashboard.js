import { A } from '@ember/array';
import Route from '@ember/routing/route';
import RSVP from 'rsvp';

import AuthenticatedRoute from '../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
  setupController(controller, model) {
    this._super(...arguments);

    controller.set('myCrates', model.myCrates);
    controller.set('myFollowing', model.myFollowing);
    controller.set('myStats', model.myStats);

    if (!controller.loadingMore) {
      controller.set('myFeed', A());
      controller.send('loadMore');
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
