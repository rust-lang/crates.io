import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class IndexRoute extends Route {
  @service fastboot;

  setupController(controller) {
    if (!controller.hasData) {
      let promise = controller.fetchData();
      if (this.fastboot.isFastBoot) {
        this.fastboot.deferRendering(promise);
      }
    }
  }
}
