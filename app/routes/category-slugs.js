import Route from '@ember/routing/route';

export default class CategorySlugsRoute extends Route {
  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  model(params) {
    return this.store.query('category-slug', params);
  }
}
