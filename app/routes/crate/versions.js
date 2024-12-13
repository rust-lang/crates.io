import Route from '@ember/routing/route';
import { waitForPromise } from '@ember/test-waiters';

export default class VersionsRoute extends Route {
  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
    // TODO: Add error handling
    waitForPromise(crate.loadVersionsTask.perform());
  }
}
