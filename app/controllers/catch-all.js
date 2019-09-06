import Controller from '@ember/controller';

export default Controller.extend({
  actions: {
    search(query) {
      return this.transitionToRoute('search', { queryParams: { q: query } });
    },
  },
});
