import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class EmailInput extends Component {
  @service notifications;

  @tracked value;
  @tracked isEditing = false;
  @tracked disableResend = false;

  @task(function* () {
    try {
      yield this.args.user.resendVerificationEmail();
      this.disableResend = true;
    } catch (error) {
      if (error.errors) {
        this.notifications.error(`Error in resending message: ${error.errors[0].detail}`);
      } else {
        this.notifications.error('Unknown error in resending message');
      }
    }
  })
  resendEmailTask;

  @action
  editEmail() {
    this.value = this.args.user.email;
    this.isEditing = true;
  }

  @task(function* (event) {
    event.preventDefault();

    let userEmail = this.value;
    let user = this.args.user;

    try {
      yield user.changeEmail(userEmail);

      this.isEditing = false;
      this.disableResend = false;
    } catch (error) {
      let msg;
      if (error.errors && error.errors[0] && error.errors[0].detail) {
        msg = `An error occurred while saving this email, ${error.errors[0].detail}`;
      } else {
        msg = 'An unknown error occurred while saving this email.';
      }
      this.notifications.error(`Error in saving email: ${msg}`);
    }
  })
  saveEmailTask;
}
