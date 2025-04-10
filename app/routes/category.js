import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class CategoryRoute extends Route {
  @service router;
  @service store;
  @service header;

  async model(params, transition) {
    let categoryName = params.category_id;
    this.header.searchValue = 'category:' + params.category_id + ' '; // additional space to help user not accidentally mangle the category

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

  deactivate() {
    super.deactivate(...arguments);
    this.header.searchValue = null;
  }
}
