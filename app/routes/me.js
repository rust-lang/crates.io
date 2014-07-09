import Ember from 'ember';

export default Ember.Route.extend({
  model: function() {
    var user = this.session.get('currentUser');
    if (user != null) {
      return user;
    } else {
      this.transitionTo('login');
    }
  }
});
