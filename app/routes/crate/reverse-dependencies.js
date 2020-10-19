import Route from '@ember/routing/route';

export default class ReverseDependenciesRoute extends Route {
  queryParams = {
    page: { refreshModel: true },
  };

  model(params) {
    params.reverse = true;
    params.crate = this.modelFor('crate');

    return this.store.query('dependency', params);
  }

  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
