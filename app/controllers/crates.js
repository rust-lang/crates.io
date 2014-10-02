import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    needs: ['application'],
    queryParams: ['letter', 'page', 'per_page', 'sort'],
    letter: 'A',
    page: '1',
    per_page: 10,
    sort: 'alpha',
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(""),
    showSortBy: false,

    selectedPage: function() { return this.get('page'); }.property('page'),

    totalItems: function() {
        return this.store.metadataFor('crate').total;
    }.property('model'),

    itemsPerPage: function() {
        return this.get('per_page');
    }.property('per_page'),

    currentSortBy: function() {
        if (this.get('sort') === 'downloads') {
            return 'Downloads today';
        } else if (this.get('sort') === 'downloads-all') {
            return 'Total Downloads';
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

