import Route from '@ember/routing/route';

export default Route.extend({
  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    return this.store.query('category', params);
  },
});
