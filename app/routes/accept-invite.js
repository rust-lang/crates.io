import Route from '@ember/routing/route';

import ajax from '../utils/ajax';

export default Route.extend({
  async model(params) {
    try {
      await ajax(`/api/v1/me/crate_owner_invitations/accept/${params.token}`, { method: 'PUT', body: '{}' });
      this.set('response', { accepted: true });
      return { response: this.response };
    } catch {
      this.set('response', { accepted: false });
      return { response: this.response };
    }
  },
});
