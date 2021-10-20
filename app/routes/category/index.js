import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CategoryIndexRoute extends Route {
  @service store;

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
    let category = this.modelFor('category');
    controller.set('category', category);
  }
}
