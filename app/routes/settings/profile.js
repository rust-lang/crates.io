import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class ProfileSettingsRoute extends AuthenticatedRoute {
  @service session;

  async model() {
    return { user: this.session.currentUser };
  }
}
