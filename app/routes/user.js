import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class UserRoute extends Route {
  @service notifications;
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  async model(params, transition) {
    const { user_id } = params;
    try {
      let user = await this.store.queryRecord('user', { user_id });

      params.user_id = user.get('id');
      params.include_yanked = 'n';
      let crates = await this.store.query('crate', params);

      return { crates, user };
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${user_id}: User not found`;
        this.router.replaceWith('catch-all', { transition, error, title });
      } else {
        let title = `${user_id}: Failed to load user data`;
        this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }
  }
}
