import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CategoryRoute extends Route {
  @service notifications;

  async model(params) {
    try {
      return await this.store.find('category', params.category_id);
    } catch (e) {
      if (e instanceof NotFoundError) {
        this.notifications.error(`Category '${params.category_id}' does not exist`);
        return this.replaceWith('index');
      }

      throw e;
    }
  }
}
