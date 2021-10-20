import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class KeywordIndexRoute extends Route {
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    params.keyword = this.modelFor('keyword').id;
    return this.store.query('crate', params);
  }

  setupController(controller) {
    controller.set('keyword', this.modelFor('keyword'));
    super.setupController(...arguments);
  }
}
