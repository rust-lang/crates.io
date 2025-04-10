import Route from '@ember/routing/route';
import { service } from '@ember/service';

import { TrackedArray } from 'tracked-built-ins';

export default class TokenListRoute extends Route {
  @service store;

  async model() {
    let apiTokens = await this.store.query('api-token', { expired_days: 30 });
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
