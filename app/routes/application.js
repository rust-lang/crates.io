import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
    beforeModel: function() {
        var self = this;
        console.log(this.session.get('isLoggedIn'),
                    this.session.get('currentUser'));
        if (this.session.get('isLoggedIn') &&
            this.session.get('currentUser') === null)
        {
            return ajax('/me').then(function(response) {
                var user = self.store.push('user', response.user);
                user.set('api_token', response.api_token);
                self.session.set('currentUser', user);
            }).catch(function() {
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
