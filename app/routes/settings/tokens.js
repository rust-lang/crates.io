import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class TokenSettingsRoute extends AuthenticatedRoute {
  @service store;

  async model() {
    let apiTokens = await this.store.findAll('api-token');
    return apiTokens.toArray();
  }
}
