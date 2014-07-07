import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    var self = this;
    return Ember.$.getJSON('/authorize', transition.queryParams, function() {
      var applicationController = self.controllerFor('application');
      var transition = applicationController.get('savedTransition');
      applicationController.loginUser();
      if (transition) {
        transition.retry();
      } else {
        self.transitionTo('index');
      }
    }).fail(function() {
      self.transitionTo('index');
    });
  },
});
