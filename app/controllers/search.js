import Ember from 'ember';
import PaginationMixin from '../mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['q', 'page', 'per_page', 'sort'],
    q: null,
    page: '1',
    per_page: 10,
    sort: null,

    totalItems: computed.readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        return (this.get('sort') === 'downloads') ? 'Downloads' : 'Relevance';
    }),

    hasItems: computed.bool('totalItems'),
});
