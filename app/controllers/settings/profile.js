import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class extends Controller {
  @service notifications;

  @tracked isAddingEmail = false;

  @tracked publishNotifications;
  @tracked notificationEmailId;

  @action handleNotificationsChange(event) {
    this.publishNotifications = event.target.checked;
  }

  @action handleNotificationEmailChange(event) {
    this.notificationEmailId = event.target.value;
  }

  updateNotificationSettings = task(async () => {
    try {
      await this.model.user.updatePublishNotifications(this.publishNotifications);
    } catch {
      this.notifications.error(
        'Something went wrong while updating your notification settings. Please try again later!',
      );
    }
  });
}
