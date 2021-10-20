import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CrateRoute extends Route {
  @service headData;
  @service notifications;
  @service store;

  async model(params) {
    try {
      return await this.store.find('crate', params.crate_id);
    } catch (error) {
      if (error.errors?.some(e => e.detail === 'Not Found')) {
        this.notifications.error(`Crate '${params.crate_id}' does not exist`);
      } else {
        this.notifications.error(`Loading data for the '${params.crate_id}' crate failed. Please try again later!`);
      }

      this.replaceWith('index');
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
