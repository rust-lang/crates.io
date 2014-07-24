import Ember from 'ember';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    var user = this.session.get('currentUser');
    if (user === null) {
      this.session.set('savedTransition', transition);
      this.controllerFor('application').set('nextFlashError',
                                            'Please log in to proceed');
      return this.transitionTo('index');
    }
  },

  model: function() {
    return this.session.get('currentUser');
  }
});
