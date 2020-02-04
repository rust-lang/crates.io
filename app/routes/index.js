import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default Route.extend({
  fetcher: service(),

  headTags() {
    return [
      {
        type: 'meta',
        attrs: {
          name: 'description',
          content: 'cargo is the package manager and crate host for rust',
        },
      },
    ];
  },

  setupController(controller, model) {
    this._super(controller, model);
    this.controllerFor('application').set('searchQuery', null);
  },

  model() {
    return this.fetcher.ajax('/api/v1/summary');
  },

  // eslint-disable-next-line no-unused-vars
  afterModel(model, transition) {
    addCrates(this.store, model.new_crates);
    addCrates(this.store, model.most_downloaded);
    addCrates(this.store, model.just_updated);
    addCrates(this.store, model.most_recently_downloaded);
  },
});

function addCrates(store, crates) {
  for (let i = 0; i < crates.length; i++) {
    crates[i] = store.push(store.normalize('crate', crates[i]));
  }
}
