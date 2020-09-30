import Component from '@ember/component';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class PendingOwnerInviteRow extends Component {
  @service notifications;

  tagName = '';

  @tracked isAccepted = false;
  @tracked isDeclined = false;

  @task(function* () {
    this.invite.set('accepted', true);

    try {
      yield this.invite.save();
      this.isAccepted = true;
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
      this.isDeclined = true;
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
