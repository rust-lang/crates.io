import Ember from 'ember';

const { inject: { service } } = Ember;

export default Ember.Controller.extend({

    ajax: service(),

    flashMessages: service(),

    isResetting: false,

    actions: {
        resetToken() {
            this.set('isResetting', true);

            this.get('ajax').put('/me/reset_token').then((d) => {
                this.get('model').set('api_token', d.api_token);
            }).catch((reason) => {
                let msg;
                if (reason.status === 403) {
                    msg = 'A login is required to perform this action';
                } else {
                    msg = 'An unknown error occurred';
                }
                this.get('flashMessages').queue(msg);
                // TODO: this should be an action, the route state machine
                // should receive signals not external transitions
                this.transitionToRoute('index');
            }).finally(() => {
                this.set('isResetting', false);
            });
        }
    }
});
