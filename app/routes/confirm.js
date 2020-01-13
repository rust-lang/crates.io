import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import ajax from '../utils/ajax';

export default Route.extend({
  flashMessages: service(),
  session: service(),
  store: service(),

  async model(params) {
    try {
      await ajax(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', body: '{}' });

      // wait for the `GET /api/v1/me` call to complete before
      // trying to update the Ember Data store
      await this.session.loadUserTask.last;

      if (this.session.currentUser) {
        this.store.pushPayload({ user: { id: this.session.currentUser.id, email_verified: true } });
      }
    } catch (error) {
      if (error.errors) {
        this.flashMessages.queue(`Error in email confirmation: ${error.errors[0].detail}`);
        return this.replaceWith('index');
      } else {
        this.flashMessages.queue(`Unknown error in email confirmation`);
        return this.replaceWith('index');
      }
    }
  },
});
