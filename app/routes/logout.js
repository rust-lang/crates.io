import Ember from 'ember';

export default Ember.Route.extend({
    activate: function() {
        var self = this;
        Ember.$.getJSON('/logout', function() {
            self.session.logoutUser();
            self.transitionTo('index');
        });
    }
});
