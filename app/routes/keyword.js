import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class KeywordRoute extends Route {
  @service notifications;
  @service router;
  @service store;

  async model({ keyword_id }) {
    try {
      return await this.store.find('keyword', keyword_id);
    } catch (error) {
      if (error instanceof NotFoundError) {
        this.notifications.error(`Keyword '${keyword_id}' does not exist`);
        return this.router.replaceWith('index');
      }

      throw error;
    }
  }
}
