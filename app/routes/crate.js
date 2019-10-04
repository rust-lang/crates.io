import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  flashMessages: service(),

  model(params) {
    return this.store.find('crate', params.crate_id).catch(e => {
      if (e.errors.some(e => e.detail === 'Not Found')) {
        this.flashMessages.show(`Crate '${params.crate_id}' does not exist`);
        return;
      }
    });
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
