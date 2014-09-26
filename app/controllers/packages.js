import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    queryParams: ['letter', 'page', 'per_page'],
    letter: 'A',
    page: '1',
    per_page: 10,
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(""),

    selectedPage: function() { return this.get('page'); }.property('page'),

    totalItems: function() {
        return this.store.metadataFor('package').total;
    }.property('model'),

    itemsPerPage: function() {
        return this.get('per_page');
    }.property('per_page'),
});

