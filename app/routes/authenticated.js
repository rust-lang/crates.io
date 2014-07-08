import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    var applicationController = this.controllerFor('application');
    if (!applicationController.get('isLoggedIn')) {
      applicationController.set('savedTransition', transition);
      this.transitionTo('login');
    }
  }
});
