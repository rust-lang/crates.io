import Route from '@ember/routing/route';
import { waitForPromise } from '@ember/test-waiters';

export default class VersionsRoute extends Route {
  queryParams = {
    sort: { refreshModel: true },
  };

  model(params) {
    // we need a model() implementation that changes, otherwise the setupController() hook
    // is not called and we won't reload the results if a new query string is used
    return params;
  }

  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    // reset when crate changes
    if (crate && crate !== controller.crate) {
      controller.reset();
    }
    controller.set('crate', crate);
    // Fetch initial data only if empty
    if (controller.data.length === 0) {
      waitForPromise(controller.loadMoreTask.perform());
    }
  }
}
