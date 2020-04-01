import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  flashMessages: service(),

  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  async model(params) {
    const { user_id } = params;
    try {
      let user = await this.store.queryRecord('user', { user_id });

      params.user_id = user.get('id');
      params.include_yanked = 'n';
      let crates = await this.store.query('crate', params);

      return { crates, user };
    } catch (e) {
      if (e.errors.some(e => e.detail === 'Not Found')) {
        this.flashMessages.queue(`User '${params.user_id}' does not exist`);
        return this.replaceWith('index');
      }
    }
  },
});
