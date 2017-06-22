import Ember from 'ember';
import PaginationMixin from '../../mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'downloads',

    totalItems: computed.readOnly('model.meta.total'),

    categoryController: Ember.inject.controller('category'),
    category: computed.alias('categoryController.model'),

    currentSortBy: computed('sort', function() {
        return (this.get('sort') === 'downloads') ? 'Downloads' : 'Alphabetical';
    }),
});
