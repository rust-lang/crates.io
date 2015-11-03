import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;

export default Ember.ArrayController.extend(PaginationMixin, {
    applicationController: Ember.inject.controller('application'),
    queryParams: ['letter', 'page', 'per_page', 'sort'],
    letter: null,
    page: '1',
    per_page: 10,
    sort: 'alpha',
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(""),
    showSortBy: false,

    totalItems: computed('model', function() {
        return this.store.metadataFor('crate').total;
    }),

    currentSortBy: computed('sort', function() {
        if (this.get('sort') === 'downloads') {
            return 'Downloads';
        } else {
            return 'Alphabetical';
        }
    }),

    actions: {
        toggleShowSortBy() {
            var opt = 'showSortBy';
            this.get('applicationController').resetDropdownOption(this, opt);
        },
    },
});

