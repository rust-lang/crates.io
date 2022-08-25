import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CategoryRoute extends Route {
  @service router;
  @service store;

  async model(params, transition) {
    let categoryName = params.category_id;

    try {
      return await this.store.findRecord('category', categoryName);
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${categoryName}: Category not found`;
        this.router.replaceWith('catch-all', { transition, title });
      } else {
        let title = `${categoryName}: Failed to load category data`;
        this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }
  }
}
