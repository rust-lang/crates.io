import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    queryParams: ['page', 'per_page'],
    page: '1',
    per_page: 10,

    totalItems: function() {
        return this.store.metadataFor('dependency').total;
    }.property('model'),
});

