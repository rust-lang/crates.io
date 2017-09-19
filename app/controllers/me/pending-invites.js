import Ember from 'ember';
import { inject as service } from '@ember/service';

export default Ember.Controller.extend({
    ajax: service(),
    isError: false,
    inviteError: 'default error message',

    actions: {
        acceptInvitation(invite) {
            this.get('ajax').put('/api/v1/me/accept_owner_invite', {
                contentType: 'application/json; charset=utf-8',
                data: JSON.stringify({
                    crate_owner_invitation: {
                        invited_by_username: invite.get('invited_by_username'),
                        crate_name: invite.get('crate_name'),
                        crate_id: invite.get('crate_id'),
                        created_at: invite.get('created_at')
                    }
                })
            }).catch((error) => {
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
