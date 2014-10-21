import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    needs: ['application'],
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'crates',
    showSortBy: false,

    totalItems: function() {
        return this.store.metadataFor('keyword').total;
    }.property('model'),

    currentSortBy: function() {
        if (this.get('sort') === 'crates') {
            return '# Crates';
        } else {
            return 'Alphabetical';
        }
    }.property('sort'),

    actions: {
        toggleShowSortBy: function() {
            var opt = 'showSortBy';
            this.get('controllers.application').resetDropdownOption(this, opt);

        },
    },
});
