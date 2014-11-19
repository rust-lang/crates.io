import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
    beforeModel: function() {
        var self = this;
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null)
        {
            return ajax('/me').then(function(response) {
                console.log("good", response);
                var user = self.store.push('user', response.user);
                user.set('api_token', response.api_token);
                self.session.set('currentUser', user);
            }).catch(function() {
                console.log('bad');
                self.session.logoutUser();
            });
        }
    },

    actions: {
        didTransition: function() {
            this.controllerFor('application').stepFlash();
        },

        willTransition: function() {
            this.controllerFor('application').aboutToTransition();
            return true;
        },
    },
});
