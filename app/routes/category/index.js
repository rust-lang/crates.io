import Route from '@ember/routing/route';

export default Route.extend({
  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    params.category = this.paramsFor('category').category_id;
    return this.store.query('crate', params);
  },

  setupController(controller) {
    this._super(...arguments);
    let category = this.modelFor('category');
    controller.set('category', category);
  },
});
