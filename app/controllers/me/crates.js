import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;
// TODO: reduce duplicatoin with controllers/crates

export default Ember.ArrayController.extend(PaginationMixin, {
    // TODO: kill needs
    needs: ['application'],
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'alpha',
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
            this.get('controllers.application').resetDropdownOption(this, opt);

        },
    },
});


