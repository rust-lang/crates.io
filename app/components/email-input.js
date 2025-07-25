import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class EmailInput extends Component {
  @service notifications;

  @tracked email = this.args.email || { email: '', id: null };
  @tracked isValid = false;
  @tracked value;
  @tracked disableResend = false;

  @action validate(event) {
    this.isValid = event.target.value.trim().length !== 0 && event.target.checkValidity();
  }

  resendEmailTask = task(async () => {
    try {
      await this.args.user.resendVerificationEmail(this.email.id);
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

  deleteEmailTask = task(async () => {
    try {
      await this.args.user.deleteEmail(this.email.id);
    } catch (error) {
      console.error('Error deleting email:', error);
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in deleting email: ${detail}`);
      } else {
        this.notifications.error('Unknown error in deleting email');
      }
    }
  });

  saveEmailTask = task(async () => {
    try {
      this.email = await this.args.user.addEmail(this.value);
      this.disableResend = true;
      await this.args.onAddEmail?.();
    } catch (error) {
      let detail = error.errors?.[0]?.detail;

      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in saving email: ${detail}`);
      } else {
        console.error('Error saving email:', error);
        this.notifications.error('Unknown error in saving email');
      }
    }
  });

  enableNotificationsTask = task(async () => {
    try {
      await this.args.user.updateNotificationEmail(this.email.id);
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in enabling notifications: ${detail}`);
      } else {
        this.notifications.error('Unknown error in enabling notifications');
      }
    }
  });
}
