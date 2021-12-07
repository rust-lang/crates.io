import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class ReverseDependenciesRoute extends Route {
  @service notifications;
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
  };

  async model(params) {
    params.reverse = true;
    params.crate = this.modelFor('crate');
    let crateName = params.crate.name;

    try {
      return await this.store.query('dependency', params);
    } catch (error) {
      let message = `Could not load reverse dependencies for the "${crateName}" crate`;

      let details = error.errors?.[0]?.detail;
      if (details && details !== '[object Object]') {
        message += `: ${details}`;
      }

      this.notifications.error(message);
      this.router.replaceWith('index');
    }
  }

  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
