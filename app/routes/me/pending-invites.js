import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class PendingInvitesRoute extends AuthenticatedRoute {
  @service store;

  queryParams = {
    seek: { refreshModel: true },
  };

  model(params) {
    return this.store.query('crate-owner-invite', params);
  }
}
