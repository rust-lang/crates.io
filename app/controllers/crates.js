import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

export default Ember.ArrayController.extend(PaginationMixin, {
    needs: ['application'],
    queryParams: ['letter', 'page', 'per_page', 'sort'],
    letter: null,
    page: '1',
    per_page: 10,
    sort: 'alpha',
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(""),
    showSortBy: false,

    totalItems: function() {
        return this.store.metadataFor('crate').total;
    }.property('model'),

    currentSortBy: function() {
        if (this.get('sort') === 'downloads') {
            return 'Downloads';
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

