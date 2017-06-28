import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Controller.extend({
    isResetting: false,

    actions: {
        resetToken() {
            this.set('isResetting', true);

            ajax({
                dataType: 'json',
                url: '/me/reset_token',
                method: 'put',
            }).then((d) => {
                this.get('model').set('api_token', d.api_token);
            }).catch((reason) => {
                let msg;
                if (reason.status === 403) {
                    msg = 'A login is required to perform this action';
                } else {
                    msg = 'An unknown error occurred';
                }
                this.controllerFor('application').set('nextFlashError', msg);
                // TODO: this should be an action, the route state machine
                // should recieve signals not external transitions
                this.transitionToRoute('index');
            }).finally(() => {
                this.set('isResetting', false);
            });
        }
    }
});
