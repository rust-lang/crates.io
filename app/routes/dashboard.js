import Ember from 'ember';
import ajax from 'ic-ajax';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    setupController: function(controller, model) {
        var self = this;
        controller.set('fetchingCrates', true);
        controller.set('fetchingFollowing', true);
        controller.set('fetchingFeed', true);
        ajax('/crates?user_id=' + model.get('id')).then(function(data) {
            controller.set('myCrates',
                           self.store.pushMany('crate', data.crates));
        }).finally(function() {
            controller.set('fetchingCrates', false);
        });

        ajax('/crates?following=1').then(function(data) {
            controller.set('myFollowing',
                           self.store.pushMany('crate', data.crates));
        }).finally(function() {
            controller.set('fetchingFollowing', false);
        });

        if (controller.get('myFeed').length === 0) {
            controller.send('loadMore');
        }
    },

    model: function() {
        return this.session.get('currentUser');
    },
});
