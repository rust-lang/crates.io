import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import ajax from '../utils/ajax';

export default class ConfirmRoute extends Route {
  @service notifications;
  @service router;
  @service session;
  @service store;

  async model(params) {
    try {
      await ajax(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', body: '{}' });

      // wait for the `GET /api/v1/me` call to complete before
      // trying to update the Ember Data store
      await this.session.loadUserTask.last;

      if (this.session.currentUser) {
        this.store.pushPayload({ user: { id: this.session.currentUser.id, email_verified: true } });
      }

      this.notifications.success('Thank you for confirming your email! :)');
    } catch (error) {
      if (error.errors) {
        this.notifications.error(`Error in email confirmation: ${error.errors[0].detail}`);
      } else {
        this.notifications.error(`Unknown error in email confirmation`);
      }
    }

    this.router.replaceWith('index');
  }
}
