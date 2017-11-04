import Controller, { inject as controller } from '@ember/controller';
import { computed } from '@ember/object';

export default Controller.extend({
    crateController: controller('crate'),
    crate: computed.alias('crateController.model'),
    error: false,
    invited: false,
    removed: false,
    username: '',

    actions: {
        addOwner() {
            this.set('error', false);
            this.set('invited', false);

            const username = this.get('username');

            if (!username) {
                this.set('error', 'Please enter a username');
                return false;
            }

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

        removeOwner(user) {
            this.set('removed', false);

            return this.get('crate').removeOwner(user.get('login')).then(() => {
                this.set('removed', `User ${user.get('login')} removed as crate owner`);

                this.get('crate.owner_user').removeObject(user);
            }).catch((error) => {
                if (error.payload) {
                    this.set('removed',
                        `Error removing owner: ${error.payload.errors[0].detail}`
                    );
                } else {
                    this.set('removed', 'Error removing owner');
                }
            });
        }
    }
});
