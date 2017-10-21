import Controller, { inject as controller } from '@ember/controller';
import { computed } from '@ember/object';

export default Controller.extend({
    crateController: controller('crate'),
    crate: computed.alias('crateController.model'),
    invited: false,

    actions: {
        addOwner(email) {
            // get user from email
            this.store.query('user', { email }).then((users) => {
                const user = users.get('firstObject');

                if (user) {
                    this.store.createRecord('crate-owner-invite', {
                        invited_by_username: email,
                        crate_name: this.get('crate.crate_name'),
                        crate_id: this.get('crate.crate_id'),
                    }).then((invite) => {
                        this.set('invited', `An invite has been sent to ${email}`);
                    }).catch((error) => {
                        if (error.payload) {
                            this.set('inviteError',
                                `Error in accepting invite: ${error.payload.errors[0].detail}`
                            );
                        } else {
                            this.set('inviteError', 'Error sending invite');
                        }
                    });
                } else {
                    // TODO: validation fail: no user
                }
            });
        },

        removeOwner(owner) {
            // TODO: remove user from crate
        }
    }
});
