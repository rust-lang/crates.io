import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  notifications: service(),

  async model(params) {
    try {
      return await this.store.find('crate', params.crate_id);
    } catch (error) {
      if (error.errors?.some(e => e.detail === 'Not Found')) {
        this.notifications.error(`Crate '${params.crate_id}' does not exist`);
        this.replaceWith('index');
      } else {
        throw error;
      }
    }
  },

  afterModel(model) {
    if (model && typeof model.get === 'function') {
      this.setHeadTags(model);
    }
  },

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
  },
});
