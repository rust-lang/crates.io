import AuthenticatedRoute from '../-authenticated-route';

export default class MeCratesRoute extends AuthenticatedRoute {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.user_id = this.session.currentUser.id;
    return this.store.query('crate', params);
  }
}
