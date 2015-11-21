import Ember from 'ember';
import AuthenticatedRoute from '../mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    data: {},

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('myCrates', this.get('data.myCrates'));
        controller.set('myFollowing', this.get('data.myFollowing'));

        if (!controller.get('loadingMore')) {
            controller.set('myFeed', []);
            controller.send('loadMore');
        }
    },

    model() {
        return this.session.get('currentUser');
    },

    afterModel(user) {
      let myCrates = this.store.query('crate', {
        user_id: user.get('id')
      });

      let myFollowing = this.store.query('crate', {
        following: 1
      });

      return Ember.RSVP.hash({
        myCrates,
        myFollowing
      }).then((hash) => this.set('data', hash) );
    }
});
