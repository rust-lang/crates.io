import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  search: service(),

  queryParams: {
    all_keywords: { refreshModel: true },
    page: { refreshModel: true },
    q: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    // we need a model() implementation that changes, otherwise the setupController() hook
    // is not called and we won't reload the results if a new query string is used
    return params;
  },

  setupController(controller, params) {
    this.search.set('inputValue', params.q);
    controller.dataTask.perform(params);
  },

  deactivate() {
    this._super(...arguments);
    this.search.set('inputValue', null);
  },
});
