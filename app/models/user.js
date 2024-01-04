import Model, { attr } from '@ember-data/model';
import { inject as service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';

import { apiAction } from '@mainmatter/ember-api-actions';

export default class User extends Model {
  @service store;

  @attr email;
  @attr email_verified;
  @attr email_verification_sent;
  @attr name;
  @attr is_admin;
  @attr login;
  @attr avatar;
  @attr url;
  @attr kind;

  async stats() {
    return await waitForPromise(apiAction(this, { method: 'GET', path: 'stats' }));
  }

  async changeEmail(email) {
    await waitForPromise(apiAction(this, { method: 'PUT', data: { user: { email } } }));

    this.store.pushPayload({
      user: {
        id: this.id,
        email,
        email_verified: false,
        email_verification_sent: true,
      },
    });
  }

  async resendVerificationEmail() {
    return await waitForPromise(apiAction(this, { method: 'PUT', path: 'resend' }));
  }
}
