import Route from '@ember/routing/route';

export default class KeywordsRoute extends Route {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    return this.store.query('keyword', params);
  }
}
