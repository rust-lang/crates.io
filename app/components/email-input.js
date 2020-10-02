import Component from '@ember/component';
import { action } from '@ember/object';
import { empty } from '@ember/object/computed';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class EmailInput extends Component {
  tagName = '';

  @service notifications;

  @tracked value;
  @tracked isEditing = false;
  @tracked disableResend = false;

  user = null;

  @empty('value') disableSave;

  @task(function* () {
    try {
      yield this.user.resendVerificationEmail();
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
    this.value = this.user.email;
    this.isEditing = true;
  }

  @action
  saveEmail() {
    let userEmail = this.value;
    let user = this.user;

    user.changeEmail(userEmail).catch(err => {
      let msg;
      if (err.errors && err.errors[0] && err.errors[0].detail) {
        msg = `An error occurred while saving this email, ${err.errors[0].detail}`;
      } else {
        msg = 'An unknown error occurred while saving this email.';
      }
      this.notifications.error(`Error in saving email: ${msg}`);
    });

    this.isEditing = false;
    this.disableResend = false;
  }
}
