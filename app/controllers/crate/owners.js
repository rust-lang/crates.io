import Controller from '@ember/controller';

export default Controller.extend({
    crate: null,
    error: false,
    invited: false,
    removed: false,
    username: '',

    actions: {
        async addOwner() {
            this.set('error', false);
            this.set('invited', false);

            const username = this.username;

            if (!username) {
                this.set('error', 'Please enter a username');
                return false;
            }

            try {
                await this.crate.inviteOwner(username);
                this.set('invited', `An invite has been sent to ${username}`);
            } catch (error) {
                if (error.payload) {
                    this.set('error', `Error sending invite: ${error.payload.errors[0].detail}`);
                } else {
                    this.set('error', 'Error sending invite');
                }
            }
        },

        async removeOwner(user) {
            this.set('removed', false);

            try {
                await this.crate.removeOwner(user.get('login'));
                this.set('removed', `User ${user.get('login')} removed as crate owner`);

                this.get('crate.owner_user').removeObject(user);
            } catch (error) {
                if (error.payload) {
                    this.set('removed', `Error removing owner: ${error.payload.errors[0].detail}`);
                } else {
                    this.set('removed', 'Error removing owner');
                }
            }
        },
    },
});
