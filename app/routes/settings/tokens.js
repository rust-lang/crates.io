import { inject as service } from '@ember/service';

import { TrackedArray } from 'tracked-built-ins';

import AuthenticatedRoute from '../-authenticated-route';

export default class TokenSettingsRoute extends AuthenticatedRoute {
  @service store;

  async model() {
    let apiTokens = await this.store.findAll('api-token');
    return TrackedArray.from(apiTokens.slice());
  }

  /**
   * Ensure that all plaintext tokens are deleted from memory after leaving
   * the API tokens settings page.
   */
  resetController(controller) {
    for (let token of controller.model) {
      if (token.token) {
        token.token = undefined;
      }
    }
  }
}
