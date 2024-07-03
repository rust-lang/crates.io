import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class TokenListRoute extends Route {
  @service router;
  @service store;

  queryParams = {
    token_id: {
      refreshModel: true,
    },
  };

  async model(params, transition) {
    const tokenId = params.token_id;
    if (tokenId) {
      try {
        return await this.store.findRecord('api-token', tokenId);
      } catch (error) {
        if (error instanceof NotFoundError) {
          let title = `Token not found`;
          this.router.replaceWith('catch-all', { transition, title });
        } else {
          let title = `Failed to load token data`;
          this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
        }
      }
    }
    return null;
  }

  setupController(controller, model) {
    super.setupController(controller, model);
    if (model) {
      const { name, endpoint_scopes, crate_scopes } = model;

      controller.name = name;
      if (endpoint_scopes) {
        controller.scopes = endpoint_scopes;
      }
      if (crate_scopes) {
        for (let pattern of crate_scopes) {
          controller.addCratePattern(pattern);
        }
      }
    }
  }

  resetController(controller) {
    controller.saveTokenTask.cancelAll();
    controller.set('token_id', null);
  }
}
