import Route from '@ember/routing/route';
import ajax from 'ember-fetch/ajax';

export default Route.extend({
  async model(params) {
    try {
      await ajax(`/api/v1/me/crate_owner_invitations/accept/${params.token}`, { method: 'PUT', body: '{}' });
      this.set('response', { accepted: true });
      return { response: this.get('response') };
    } catch (error) {
      this.set('response', { accepted: false });
      return { response: this.get('response') };
    }
  },
});
