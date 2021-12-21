import Controller from '@ember/controller';
import { action } from '@ember/object';
import Ember from 'ember';

import { alias } from 'macro-decorators';

import ajax from '../../utils/ajax';

export default class EmailNotificationsSettingsController extends Controller {
  isResetting = false;

  @alias('model.ownedCrates') ownedCrates;

  emailNotificationsError = false;
  emailNotificationsSuccess = false;

  get hasEmailNotificationFeature() {
    return Ember.testing;
  }

  setAllEmailNotifications(value) {
    this.ownedCrates.forEach(c => {
      c.set('email_notifications', value);
    });
  }

  @action
  async saveEmailNotifications() {
    try {
      await ajax(`/api/v1/me/email_notifications`, {
        method: 'PUT',
        body: JSON.stringify(
          this.ownedCrates.map(c => ({
            id: parseInt(c.id, 10),
            email_notifications: c.email_notifications,
          })),
        ),
      });
      this.setProperties({
        emailNotificationsError: false,
        emailNotificationsSuccess: true,
      });
    } catch (error) {
      console.error(error);
      this.setProperties({
        emailNotificationsError: true,
        emailNotificationsSuccess: false,
      });
    }
  }

  @action
  emailNotificationsSelectAll() {
    this.setAllEmailNotifications(true);
  }

  @action
  emailNotificationsSelectNone() {
    this.setAllEmailNotifications(false);
  }
}
