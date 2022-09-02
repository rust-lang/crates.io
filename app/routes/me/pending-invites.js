import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class PendingInvitesRoute extends AuthenticatedRoute {
  @service store;

  queryParams = {
    page: { refreshModel: true },
  };

  model() {
    return this.store.findAll('crate-owner-invite');
  }
}
