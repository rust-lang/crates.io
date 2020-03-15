import Route from '@ember/routing/route';

export default class CategoryIndexRoute extends Route {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.category = this.paramsFor('category').category_id;
    return this.store.query('crate', params);
  }

  setupController(controller) {
    super.setupController(...arguments);

    // TODO: move to model hook
    let category = this.modelFor('category');
    controller.set('category', category);
  }
}
