import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class KeywordIndexRoute extends Route {
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  async model(params, transition) {
    let { keyword_id: keyword, page, sort } = params;

    try {
      let crates = await this.store.query('crate', { keyword, page, sort });
      return { keyword, crates };
    } catch (error) {
      let title = `${keyword}: Failed to load crates`;
      this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }
  }
}
