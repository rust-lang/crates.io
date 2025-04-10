import { service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class PendingInvitesRoute extends AuthenticatedRoute {
  @service session;
  @service store;

  model() {
    let user = this.session.currentUser;
    return this.store.query('crate-owner-invite', { invitee_id: user.id });
  }
}
