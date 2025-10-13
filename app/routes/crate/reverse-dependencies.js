import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class ReverseDependenciesRoute extends Route {
  @service notifications;
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
  };

  async model(params, transition) {
    params.reverse = true;
    params.crate = this.modelFor('crate');
    let crateName = params.crate.name;

    try {
      return await this.store.query('dependency', params);
    } catch (error) {
      let title = `${crateName}: Failed to load dependents`;
      this.router.replaceWith('catch-all', { transition, error, title });
    }
  }

  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
