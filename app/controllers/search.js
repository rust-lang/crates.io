import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    queryParams: ['q', 'page', 'per_page'],
    q: null,
    page: '1',
    per_page: 10,

    selectedPage: function() { return this.get('page'); }.property('page'),

    totalItems: function() {
        return this.store.metadataFor('crate').total;
    }.property('model'),

    itemsPerPage: function() {
        return this.get('per_page');
    }.property('per_page'),
});
