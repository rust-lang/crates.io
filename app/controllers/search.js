import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    queryParams: ['q', 'page', 'per_page'],
    q: null,
    page: '1',
    per_page: 10,

    name: function() {
      return this.get("q") + " - Cargo search";
    }.property('model'),

    totalItems: function() {
        return this.store.metadataFor('crate').total;
    }.property('model'),
});
