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
        },
        declineInvitation(invite) {
            this.get('ajax').put('api/v1/me/decline_owner_invite', {
                contentType: 'application/json; charset=utf-8',
                data: JSON.stringify({
                    crate_owner_invitation: {
                        invited_by_username: invite.get('invited_by_username'),
                        crate_name: invite.get('crate_name'),
                        crate_id: invite.get('crate_id'),
                        created_at: invite.get('created_at')
                    }
                })
            }).then(() => {
                this.get('ajax').request('/api/v1/me/crate_owner_invitations').then((response) => {
                    this.set('model', this.store.push(this.store.normalize('crate-owner-invite', response.crate_owner_invite)));
                });
            }).catch((error) => {
                this.set('isError', true);
                if (error.payload) {
                    this.set('inviteError', `Error in declining invite: ${error.payload.errors[0].detail}`);
                } else {
                    this.set('inviteError', 'Error in accepting invite');
                }
            });
        }
    }
});
