import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    var self = this;
    transition.queryParams.code += 'wut';
    return Ember.$.getJSON('/authorize', transition.queryParams, function(d) {
      if (!d.ok) {
        self.controllerFor('application').setFlashError(d.error);
        self.transitionTo('index');
        return;
      }

      var applicationController = self.controllerFor('application');
      var transition = applicationController.get('savedTransition');
      applicationController.loginUser();
      if (transition) {
        transition.retry();
      } else {
        self.transitionTo('index');
      }
    });
  },
});
