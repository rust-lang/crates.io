import Ember from 'ember';
import PaginationMixin from 'cargo/mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['letter', 'page', 'per_page', 'sort'],
    letter: null,
    page: '1',
    per_page: 10,
    sort: 'alpha',
    alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(""),

    totalItems: computed.readOnly('model.meta.total'),

    currentSortBy: computed('sort', function() {
        return (this.get('sort') === 'downloads') ? 'Downloads' : 'Alphabetical';
    }),
});
