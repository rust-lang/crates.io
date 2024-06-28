import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import { patternDescription } from '../../../utils/token-scopes';

export default class TokenListRoute extends Route {
  @service store;

  queryParams = {
    token_id: {
      refreshModel: true,
    },
  };

  async model(params) {
    const tokenId = params.token_id;
    if (tokenId) {
      return await this.store.findRecord('api-token', tokenId);
    }
    return null;
  }

  setupController(controller, model) {
    super.setupController(controller, model);
    if (model) {
      const { name, endpoint_scopes, crate_scopes } = model;
      let properties = {
        name,
        ...(endpoint_scopes && { scopes: endpoint_scopes }),
        ...(crate_scopes && {
          crateScopes: crate_scopes.map(pattern => ({
            pattern,
            showAsInvalid: false,
            description: patternDescription(pattern),
          })),
        }),
      };

      controller.setProperties(properties);
    }
  }

  resetController(controller) {
    controller.saveTokenTask.cancelAll();
  }
}
