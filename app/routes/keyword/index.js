import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class KeywordIndexRoute extends Route {
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  async model(params, transition) {
    let keyword = this.modelFor('keyword');
    try {
      return await this.store.query('crate', { ...params, keyword });
    } catch (error) {
      let title = `${keyword}: Failed to load crates`;
      this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }
  }

  setupController(controller) {
    controller.set('keyword', this.modelFor('keyword'));
    super.setupController(...arguments);
  }
}
