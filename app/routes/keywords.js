import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class KeywordsRoute extends Route {
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    return this.store.query('keyword', params);
  }
}
