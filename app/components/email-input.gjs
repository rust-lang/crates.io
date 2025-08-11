import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class EmailInput extends Component {
  @service notifications;

  @tracked value;
  @tracked isEditing = false;
  @tracked disableResend = false;

  resendEmailTask = task(async () => {
    try {
      await this.args.user.resendVerificationEmail();
      this.disableResend = true;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in resending message: ${detail}`);
      } else {
        this.notifications.error('Unknown error in resending message');
      }
    }
  });

  @action
  editEmail() {
    this.value = this.args.user.email;
    this.isEditing = true;
  }

  saveEmailTask = task(async () => {
    let userEmail = this.value;
    let user = this.args.user;

    try {
      await user.changeEmail(userEmail);

      this.isEditing = false;
      this.disableResend = false;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;

      let msg =
        detail && !detail.startsWith('{')
          ? `An error occurred while saving this email, ${detail}`
          : 'An unknown error occurred while saving this email.';

      this.notifications.error(`Error in saving email: ${msg}`);
    }
  });
}
