import Ember from 'ember';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    data: {},

    setupController: function(controller, model) {
        this._super(controller, model);
        controller.set('fetchingFeed', true);
        controller.set('myCrates', this.get('data.myCrates'));
        controller.set('myFollowing', this.get('data.myFollowing'));
        if (!controller.get('loadingMore')) {
            controller.set('myFeed', []);
            controller.send('loadMore');
        }
    },

    model: function() {
        return this.session.get('currentUser');
    },

    afterModel: function(user) {
        var self = this;
        return Ember.RSVP.hash({
            myCrates: this.store.find('crate', {user_id: user.get('id')}),
            myFollowing: this.store.find('crate', {following: 1}),
        }).then(function(hash) {
            self.set('data', hash);
        });
    },
});
