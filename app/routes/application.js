import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function() {
    var self = this;
    if (this.session.get('isLoggedIn') &&
        this.session.get('currentUser') === null) {
      return Ember.$.getJSON('/me').then(function(d) {
        self.session.set('currentUser', d.user);
      }).fail(function() {
        self.session.logoutUser();
      });
    }
  },

  actions: {
    didTransition: function() {
      this.controllerFor('application').stepFlash();
    },
  },
});
