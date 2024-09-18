import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class extends Controller {
  @service notifications;

  @tracked publishNotifications;

  @action handleNotificationsChange(event) {
    this.publishNotifications = event.target.checked;
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
