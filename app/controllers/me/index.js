import Controller from '@ember/controller';
import { action, computed } from '@ember/object';
import { notEmpty, filterBy, sort, alias } from '@ember/object/computed';
import Ember from 'ember';

import ajax from '../../utils/ajax';

export default class MeIndexController extends Controller {
  // eslint-disable-next-line ember/avoid-leaking-state-in-ember-objects
  tokenSort = ['created_at:desc'];

  @sort('model.api_tokens', 'tokenSort') sortedTokens;

  isResetting = false;

  @alias('model.ownedCrates') ownedCrates;

  @filterBy('model.api_tokens', 'isNew', true) newTokens;

  @notEmpty('newTokens') disableCreate;

  emailNotificationsError = false;
  emailNotificationsSuccess = false;

  @computed
  get hasEmailNotificationFeature() {
    return Ember.testing;
  }

  setAllEmailNotifications(value) {
    this.ownedCrates.forEach(c => {
      c.set('email_notifications', value);
    });
  }

  @action
  async saveEmailNotifications(event) {
    event?.preventDefault();

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

  @action
  startNewToken() {
    this.store.createRecord('api-token', {
      created_at: new Date(Date.now() + 2000),
    });
  }
}
