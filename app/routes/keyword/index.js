import Route from '@ember/routing/route';

export default Route.extend({
  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    params.keyword = this.modelFor('keyword').id;
    return this.store.query('crate', params);
  },

  setupController(controller) {
    controller.set('keyword', this.modelFor('keyword'));
    this._super(...arguments);
  },
});
