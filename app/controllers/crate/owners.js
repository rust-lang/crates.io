import Controller, { inject as controller } from '@ember/controller';
import { computed } from '@ember/object';

export default Controller.extend({
    crateController: controller('crate'),
    crate: computed.alias('crateController.model'),
    error: false,
    invited: false,
    username: '',

    actions: {
        addOwner() {
            const username = this.get('username');

            if (!username) {
                this.set('error', 'Please enter a username');
                return false;
            }

            this.set('error', false);

            return this.get('crate').inviteOwner(username).then(() => {
                this.set('invited', `An invite has been sent to ${username}`);
            }).catch((error) => {
                if (error.payload) {
                    this.set('error',
                        `Error sending invite: ${error.payload.errors[0].detail}`
                    );
                } else {
                    this.set('error', 'Error sending invite');
                }
            });
        },

        removeOwner(username) {
            this.set('error', false);

            return this.get('crate').removeOwner(username).then(() => {
                // TODO: update DOM
            }).catch(() => {
                // TODO: show error
            });
        }
    }
});
