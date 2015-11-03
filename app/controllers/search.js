import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['q', 'page', 'per_page'],
    q: null,
    page: '1',
    per_page: 10,

    name: computed('model', function() {
      return this.get("q") + " - Cargo search";
    }),

    totalItems: computed('model', function() {
        return this.store.metadataFor('crate').total;
    })
});
