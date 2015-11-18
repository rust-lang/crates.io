import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;
// TODO: reduce duplicatoin with controllers/me/crates

export default Ember.Controller.extend(PaginationMixin, {
    applicationController: Ember.inject.controller('application'),
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'alpha',
    showSortBy: false,

    totalItems: computed('model', function() {
        return this.get('model.meta').total;
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
