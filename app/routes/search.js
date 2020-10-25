import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class SearchRoute extends Route {
  @service header;

  queryParams = {
    all_keywords: { refreshModel: true },
    page: { refreshModel: true },
    q: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    // we need a model() implementation that changes, otherwise the setupController() hook
    // is not called and we won't reload the results if a new query string is used
    return params;
  }

  setupController(controller, params) {
    this.header.searchValue = params.q;
    controller.fetchData();
  }

  deactivate() {
    super.deactivate(...arguments);
    this.header.searchValue = null;
  }
}
