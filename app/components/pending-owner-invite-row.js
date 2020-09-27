import Component from '@ember/component';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';

export default class PendingOwnerInviteRow extends Component {
  @service notifications;

  tagName = '';

  isAccepted = false;
  isDeclined = false;

  @task(function* () {
    this.invite.set('accepted', true);

    try {
      yield this.invite.save();
      this.set('isAccepted', true);
    } catch (error) {
      if (error.errors) {
        this.notifications.error(`Error in accepting invite: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Error in accepting invite');
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
      if (error.errors) {
        this.notifications.error(`Error in declining invite: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Error in declining invite');
      }
    }
  })
  declineInvitationTask;
}
