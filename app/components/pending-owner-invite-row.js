import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class PendingOwnerInviteRow extends Component {
  @service notifications;

  @tracked isAccepted = false;
  @tracked isDeclined = false;

  @task *acceptInvitationTask() {
    this.args.invite.set('accepted', true);

    try {
      yield this.args.invite.save();
      this.isAccepted = true;
    } catch (error) {
      if (error.errors?.[0]?.detail && error.errors[0].detail !== '[object Object]') {
        this.notifications.error(`Error in accepting invite: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Error in accepting invite');
      }
    }
  }

  @task *declineInvitationTask() {
    this.args.invite.set('accepted', false);

    try {
      yield this.args.invite.save();
      this.isDeclined = true;
    } catch (error) {
      if (error.errors?.[0]?.detail && error.errors[0].detail !== '[object Object]') {
        this.notifications.error(`Error in declining invite: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Error in declining invite');
      }
    }
  }
}
