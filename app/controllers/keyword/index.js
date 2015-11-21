import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'alpha',

    totalItems: computed.readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        if (this.get('sort') === 'downloads') {
            return 'Downloads';
        } else {
            return 'Alphabetical';
        }
    }),
});
