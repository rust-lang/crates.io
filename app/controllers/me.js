import Ember from 'ember';

export default Ember.ObjectController.extend({
  actions: {
    resetToken: function() {
      var self = this;
      Ember.$.ajax({
        dataType: "json",
        url: '/reset_token',
        method: 'put',
      }).then(function(d) {
        self.get('model').set('api_token', d.api_token);
      }).fail(function(reason) {
        var msg;
        if (reason.status === 403) {
          msg = "A login is required to perform this action";
        } else {
          msg = "An unknown error occurred";
        }
        self.controllerFor('application').setFlashError(msg);
        self.transitionToRoute('index');
      });
    }
  }
});
