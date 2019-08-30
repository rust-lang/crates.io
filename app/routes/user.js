import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import RSVP from 'rsvp';

export default Route.extend({
  flashMessages: service(),

  queryParams: {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    const { user_id } = params;
    return this.store.queryRecord('user', { user_id }).then(
      user => {
        params.user_id = user.get('id');
        params.include_yanked = 'n';
        return RSVP.hash({
          crates: this.store.query('crate', params),
          user,
        });
      },
      e => {
        if (e.errors.some(e => e.detail === 'Not Found')) {
          this.flashMessages.queue(`User '${params.user_id}' does not exist`);
          return this.replaceWith('index');
        }
      },
    );
  },
});
