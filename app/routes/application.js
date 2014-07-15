import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  beforeModel: function() {
    var self = this;
    if (this.session.get('isLoggedIn') &&
        this.session.get('currentUser') === null) {
      return ajax('/me').then(function(d) {
        self.session.set('currentUser', d.user);
      }).catch(function() {
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
