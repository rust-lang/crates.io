import AuthenticatedRoute from '../-authenticated-route';

export default class FollowingRoute extends AuthenticatedRoute {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.following = 1;
    return this.store.query('crate', params);
  }
}
