import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  notifications: service(),

  async model({ keyword_id }) {
    try {
      return await this.store.find('keyword', keyword_id);
    } catch (e) {
      if (e instanceof NotFoundError) {
        this.notifications.error(`Keyword '${keyword_id}' does not exist`);
        return this.replaceWith('index');
      }

      throw e;
    }
  },
});
