import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CratesRoute extends Route {
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    return this.store.query('crate', params);
  }
}
