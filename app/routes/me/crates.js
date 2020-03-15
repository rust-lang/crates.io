import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default class MeCratesRoute extends Route.extend(AuthenticatedRoute) {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.user_id = this.get('session.currentUser.id');
    return this.store.query('crate', params);
  }
}
