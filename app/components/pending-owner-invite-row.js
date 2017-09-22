import Ember from 'ember';

export default Ember.Component.extend({
    isSuccess: false,
    isError: false,
    inviteError: 'default error message',

    actions: {
        acceptInvitation(invite) {
            invite.set('accepted', true);
            invite.save()
                .then(() => {
                    this.set('isSuccess', true);
                })
                .catch((error) => {
                    this.set('isError', true);
                    if (error.payload) {
                        this.set('inviteError',
                            `Error in accepting invite: ${error.payload.errors[0].detail}`
                        );
                    } else {
                        this.set('inviteError', 'Error in accepting invite');
                    }
                });
        }
    }
});
