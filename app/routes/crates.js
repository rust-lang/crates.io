import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class CratesRoute extends Route {
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  async model(params, transition) {
    try {
      return await this.store.query('crate', params);
    } catch (error) {
      let title = `Failed to load crate list`;
      let details = error.errors?.[0]?.detail;
      return this.router.replaceWith('catch-all', { transition, error, title, details, tryAgain: true });
    }
  }
}
