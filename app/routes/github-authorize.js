import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    var self = this;
    return Ember.$.getJSON('/authorize', transition.queryParams, function(d) {
      if (!d.ok) {
        self.controllerFor('application').setFlashError(d.error);
        self.transitionTo('index');
        return;
      }

      var transition = self.session.get('savedTransition');
      self.session.loginUser();
      if (transition) {
        transition.retry();
      } else {
        self.transitionTo('index');
      }
    });
  },
});
