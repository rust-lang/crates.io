import Ember from 'ember';

export default Ember.ObjectController.extend({
  needs: ['application'],
  actions: {
    updatePackage: function() {
      var self = this;
      this.get('model').save().then(function(pkg) {
        self.transitionToRoute('package', pkg);
      }).catch(function(reason) {
        self.get('model').rollback();
        var msg;
        if (reason.status === 403) {
          msg = "A login is required to perform that action";
        } else {
          msg = "An unknown error occurred";
        }
        self.controllerFor('application').setFlashError(msg);
        self.transitionToRoute('index');
      });
    }
  }
});
