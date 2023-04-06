import { inject as service } from '@ember/service';

import { TrackedArray } from 'tracked-built-ins';

import AuthenticatedRoute from '../-authenticated-route';

export default class TokenSettingsRoute extends AuthenticatedRoute {
  @service store;

  async model() {
    let apiTokens = await this.store.findAll('api-token');
    return TrackedArray.from(apiTokens.slice());
  }
}
