import Route from '@ember/routing/route';
import { service } from '@ember/service';

import ajax from '../utils/ajax';

export default class ConfirmRoute extends Route {
  @service notifications;
  @service router;
  @service session;
  @service store;

  async model(params) {
    try {
      let response = await ajax(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', body: '{}' });

      // wait for the `GET /api/v1/me` call to complete before
      // trying to update the Ember Data store
      await this.session.loadUserTask.last;

      if (this.session.currentUser) {
        this.store.pushPayload({
          user: {
            id: this.session.currentUser.id,
            emails: [
              ...this.session.currentUser.emails.filter(email => email.id !== response.email.id),
              response.email,
            ].sort((a, b) => a.id - b.id),
          },
        });
      }

      this.notifications.success('Thank you for confirming your email! :)');
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in email confirmation: ${detail}`);
      } else {
        this.notifications.error(`Unknown error in email confirmation`);
      }
    }

    this.router.replaceWith('index');
  }
}
