import Route from '@ember/routing/route';

import ajax from '../utils/ajax';

export default class AcceptInviteRoute extends Route {
  async model(params) {
    try {
      await ajax(`/api/v1/me/crate_owner_invitations/accept/${params.token}`, { method: 'PUT', body: '{}' });
      return { response: { accepted: true } };
    } catch {
      return { response: { accepted: false } };
    }
  }
}
