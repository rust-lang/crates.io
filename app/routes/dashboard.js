import Route from '@ember/routing/route';
import { A } from '@ember/array';
import RSVP from 'rsvp';

import AuthenticatedRoute from '../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
  setupController(controller) {
    this._super(...arguments);

    controller.set('myCrates', this.get('data.myCrates'));
    controller.set('myFollowing', this.get('data.myFollowing'));
    controller.set('myStats', this.get('data.myStats'));

    if (!controller.loadingMore) {
      controller.set('myFeed', A());
      controller.send('loadMore');
    }
  },

  model() {
    return this.get('session.currentUser');
  },

  async afterModel(user) {
    let myCrates = this.store.query('crate', {
      user_id: user.get('id'),
    });

    let myFollowing = this.store.query('crate', {
      following: 1,
    });

    let myStats = user.stats();

    this.set('data', await RSVP.hash({ myCrates, myFollowing, myStats }));
  },
});
