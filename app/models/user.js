import Model, { attr } from '@ember-data/model';
import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';

import { apiAction } from '@mainmatter/ember-api-actions';

export default class User extends Model {
  @service store;

  @attr emails;
  @attr name;
  @attr is_admin;
  @attr login;
  @attr avatar;
  @attr url;
  @attr kind;
  @attr publish_notifications;

  async stats() {
    return await waitForPromise(apiAction(this, { method: 'GET', path: 'stats' }));
  }

  async addEmail(emailAddress) {
    let email = await waitForPromise(
      apiAction(this, {
        method: 'POST',
        path: 'emails',
        data: { email: emailAddress },
      }),
    );

    this.store.pushPayload({
      user: {
        id: this.id,
        emails: [...this.emails, email],
      },
    });
  }

  async resendVerificationEmail(emailId) {
    return await waitForPromise(apiAction(this, { method: 'PUT', path: `emails/${emailId}/resend` }));
  }

  async deleteEmail(emailId) {
    await waitForPromise(apiAction(this, { method: 'DELETE', path: `emails/${emailId}` }));

    this.store.pushPayload({
      user: {
        id: this.id,
        emails: this.emails.filter(email => email.id !== emailId),
      },
    });
  }

  async updateNotificationEmail(emailId) {
    await waitForPromise(apiAction(this, { method: 'PUT', path: `emails/${emailId}/notifications` }));

    this.store.pushPayload({
      user: {
        id: this.id,
        emails: this.emails.map(email => ({ ...email, send_notifications: email.id === emailId })),
      },
    });
  }

  async updatePublishNotifications(enabled) {
    await waitForPromise(apiAction(this, { method: 'PUT', data: { user: { publish_notifications: enabled } } }));

    this.store.pushPayload({
      user: {
        id: this.id,
        publish_notifications: enabled,
      },
    });
  }
}
