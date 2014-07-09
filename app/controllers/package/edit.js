import Ember from 'ember';

export default Ember.ObjectController.extend({
  actions: {
    updatePackage: function() {
      this.get('model').save();
      this.transitionTo('package', this.get('model'));
    }
  }
});
