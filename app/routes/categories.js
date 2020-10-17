import Route from '@ember/routing/route';

export default class CategoriesRoute extends Route {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    return this.store.query('category', params);
  }
}
