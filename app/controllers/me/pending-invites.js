import Ember from 'ember';
import { inject as service } from '@ember/service';

export default Ember.Controller.extend({
    ajax: service(),
    isSuccess: false,
    isError: false,
    inviteError: 'default error message',

    actions: {
        acceptInvitation(invite) {
            invite.set('accepted', true);
            invite.save()
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
