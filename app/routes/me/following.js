import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class FollowingRoute extends AuthenticatedRoute {
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.following = 1;
    return this.store.query('crate', params);
  }
}
