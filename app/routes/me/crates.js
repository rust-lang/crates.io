import AuthenticatedRoute from '../-authenticated-route';

export default AuthenticatedRoute.extend({
  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    params.user_id = this.get('session.currentUser.id');
    return this.store.query('crate', params);
  },
});
