import Ember from 'ember';

export default Ember.Route.extend({
  actions: {
    didTransition: function() {
      console.log('wut');
      this.controllerFor('application').stepFlash();
    },
  },
});
