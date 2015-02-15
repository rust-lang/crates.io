import Ember from 'ember';

export default Ember.Route.extend({
    beforeModel: function(transition) {
        try { localStorage.removeItem('github_response'); } catch (e) {}
        delete window.github_response;
        var win = window.open('/github_login', 'Authorization',
                              'width=1000,height=450,' +
                              'toolbar=0,scrollbars=1,status=1,resizable=1,' +
                              'location=1,menuBar=0');
        if (!win) { return; }

        // For the life of me I cannot figure out how to do this other than
        // polling
        var self = this;
        var oauthInterval = window.setInterval(function(){
            if (!win.closed) { return; }
            window.clearInterval(oauthInterval);
            var json = window.github_response;
            delete window.github_response;
            if (!json) { return; }

            var response = JSON.parse(json);
            if (!response) { return; }
            if (!response.ok) {
                self.controllerFor('application').set('flashError',
                                                      'Failed to log in');
                return;
            }
            var data = response.data;
            if (data.errors) {
                var error = "Failed to log in: " + data.errors[0].detail;
                self.controllerFor('application').set('flashError', error);
                return;
            }

            var user = self.store.push('user', data.user);
            user.set('api_token', data.api_token);
            var transition = self.session.get('savedTransition');
            self.session.loginUser(user);
            if (transition) {
                transition.retry();
            }
        }, 200);
        transition.abort();
    }
});

