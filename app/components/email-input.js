import Component from '@ember/component';
import { action, computed } from '@ember/object';
import { empty } from '@ember/object/computed';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class EmailInput extends Component {
  tagName = '';

  value = '';
  @tracked isEditing = false;
  user = null;

  @empty('user.email') disableSave;

  prevEmail = '';

  @computed('user.email')
  get emailIsNull() {
    let email = this.get('user.email');
    return email == null;
  }

  @computed('user.{email,email_verified}')
  get emailNotVerified() {
    let email = this.get('user.email');
    let verified = this.get('user.email_verified');

    return email != null && !verified;
  }

  isError = false;
  emailError = '';
  disableResend = false;

  @computed('disableResend', 'user.email_verification_sent')
  get resendButtonText() {
    if (this.disableResend) {
      return 'Sent!';
    } else if (this.get('user.email_verification_sent')) {
      return 'Resend';
    } else {
      return 'Send verification email';
    }
  }

  @task(function* () {
    try {
      yield this.user.resendVerificationEmail();
      this.set('disableResend', true);
    } catch (error) {
      if (error.errors) {
        this.set('isError', true);
        this.set('emailError', `Error in resending message: ${error.errors[0].detail}`);
      } else {
        this.set('isError', true);
        this.set('emailError', 'Unknown error in resending message');
      }
    }
  })
  resendEmailTask;

  @action
  editEmail() {
    let email = this.value;
    this.set('emailIsNull', email == null);
    this.isEditing = true;
    this.set('prevEmail', this.value);
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
      user.set('email', this.prevEmail);
      this.set('isError', true);
      this.set('emailError', `Error in saving email: ${msg}`);
    });

    this.isEditing = false;
    this.set('disableResend', false);
  }

  @action
  cancelEdit() {
    this.isEditing = false;
    this.set('value', this.prevEmail);
  }
}
