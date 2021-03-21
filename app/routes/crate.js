import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CrateRoute extends Route {
  @service notifications;

  async model(params) {
    try {
      return await this.store.find('crate', params.crate_id);
    } catch (error) {
      if (error.errors?.some(e => e.detail === 'Not Found')) {
        this.notifications.error(`Crate '${params.crate_id}' does not exist`);
      } else {
        this.notifications.error(`Loading data for the '${params.crate_id}' crate failed. Please try again later!`);
      }

      this.replaceWith('index');
    }
  }

  afterModel(model) {
    if (model && typeof model.get === 'function') {
      this.setHeadTags(model);
    }
  }

  setHeadTags(model) {
    let headTags = [
      {
        type: 'meta',
        tagId: 'meta-description-tag',
        attrs: {
          name: 'description',
          content: model.get('description') || 'A package for Rust.',
        },
      },
    ];

    this.set('headTags', headTags);
  }
}
