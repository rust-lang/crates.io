import Component from '@ember/component';
import { inject as service } from '@ember/service';

export default Component.extend({
  header: service(),
  router: service(),
  session: service(),

  tagName: '',

  actions: {
    search(event) {
      event.preventDefault();

      this.router.transitionTo('search', {
        queryParams: {
          q: this.header.searchValue,
          page: 1,
        },
      });
    },
  },
});
