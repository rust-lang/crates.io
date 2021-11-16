import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CrateRoute extends Route {
  @service headData;
  @service router;
  @service store;

  async model(params, transition) {
    try {
      return await this.store.find('crate', params.crate_id);
    } catch (error) {
      if (error.errors?.some(e => e.detail === 'Not Found')) {
        this.router.replaceWith('catch-all', { transition, error, title: 'Crate not found' });
      } else {
        this.router.replaceWith('catch-all', { transition, error, title: 'Crate failed to load', tryAgain: true });
      }
    }
  }

  setupController(controller, model) {
    super.setupController(...arguments);
    this.headData.crate = model;
  }

  resetController() {
    super.resetController(...arguments);
    this.headData.crate = null;
  }
}
