import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.ObjectController.extend({
    isResetting: false,

    actions: {
        resetToken: function() {
            this.set('isResetting', true);
            var self = this;
            ajax({
                dataType: "json",
                url: '/me/reset_token',
                method: 'put',
            }).then(function(d) {
                self.get('model').set('api_token', d.api_token);
            }).catch(function(reason) {
                var msg;
                if (reason.status === 403) {
                    msg = "A login is required to perform this action";
                } else {
                    msg = "An unknown error occurred";
                }
                self.controllerFor('application').set('nextFlashError', msg);
                self.transitionToRoute('index');
            }).finally(function() {
                self.set('isResetting', false);
            });
        }
    }
});
