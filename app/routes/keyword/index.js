import Route from '@ember/routing/route';

export default class KeywordIndexRoute extends Route {
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
