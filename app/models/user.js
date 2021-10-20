import Model, { attr } from '@ember-data/model';
import { inject as service } from '@ember/service';

import { memberAction } from 'ember-api-actions';

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

  stats = memberAction({ type: 'GET', path: 'stats' });

  async changeEmail(email) {
    await this.#changeEmail(email);
    this.store.pushPayload({
      user: {
        id: this.id,
        email,
        email_verified: false,
        email_verification_sent: true,
      },
    });
  }

  #changeEmail = memberAction({
    type: 'PUT',
    before(email) {
      return { user: { email } };
    },
  });

  resendVerificationEmail = memberAction({
    type: 'PUT',
    path: 'resend',
  });
}
