import Model, { attr } from '@ember-data/model';
import { inject as service } from '@ember/service';

import { customAction } from '../utils/custom-action';

export default class User extends Model {
  @service store;

  @attr email;
  @attr email_verified;
  @attr email_verification_sent;
  @attr name;
  @attr login;
  @attr avatar;
  @attr url;
  @attr kind;
  @attr admin;

  async stats() {
    return await customAction(this, { method: 'GET', path: 'stats' });
  }

  async changeEmail(email) {
    await customAction(this, { method: 'PUT', data: { user: { email } } });

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
    return await customAction(this, { method: 'PUT', path: 'resend' });
  }
}
