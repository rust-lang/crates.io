import Route from '@ember/routing/route';

export default Route.extend({
  queryParams: {
    letter: { refreshModel: true },
    page: { refreshModel: true },
    sort: { refreshModel: true },
  },

  model(params) {
    // The backend throws an error if the letter param is
    // empty or null.
    if (!params.letter) {
      delete params.letter;
    }

    return this.store.query('crate', params);
  },
});
