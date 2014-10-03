import Ember from 'ember';
import ajax from 'ic-ajax';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    setupController: function(controller, model) {
        var self = this;
        controller.set('fetchingCrates', true);
        controller.set('fetchingFollowing', true);
        ajax('/crates?user_id=' + model.get('id')).then(function(data) {
            data.crates = Ember.A(data.crates);
            data.crates.forEach(function(crate, i, arr) {
                arr[i] = self.store.push('crate', crate);
            });
            controller.set('myCrates', data.crates);
        }).finally(function() {
            controller.set('fetchingCrates', false);
        });
        ajax('/crates?following=1').then(function(data) {
            Ember.A(data.crates).forEach(function(crate, i, arr) {
                arr[i] = self.store.push('crate', crate);
            });
            controller.set('myFollowing', data.crates);
        }).finally(function() {
            controller.set('fetchingFollowing', false);
        });
    },

    model: function() {
        return this.session.get('currentUser');
    },
});
