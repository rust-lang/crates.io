import Component from '@ember/component';

import { task } from 'ember-concurrency';

export default class PendingOwnerInviteRow extends Component {
  tagName = '';

  isAccepted = false;
  isDeclined = false;
  isError = false;
  inviteError = 'default error message';

  @task(function* () {
    this.invite.set('accepted', true);

    try {
      yield this.invite.save();
      this.set('isAccepted', true);
    } catch (error) {
      this.set('isError', true);
      if (error.errors) {
        this.set('inviteError', `Error in accepting invite: ${error.errors[0].detail}`);
      } else {
        this.set('inviteError', 'Error in accepting invite');
      }
    }
  })
  acceptInvitationTask;

  @task(function* () {
    this.invite.set('accepted', false);

    try {
      yield this.invite.save();
      this.set('isDeclined', true);
    } catch (error) {
      this.set('isError', true);
      if (error.errors) {
        this.set('inviteError', `Error in declining invite: ${error.errors[0].detail}`);
      } else {
        this.set('inviteError', 'Error in declining invite');
      }
    }
  })
  declineInvitationTask;
}
