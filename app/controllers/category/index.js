import Controller, { inject as controller } from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page', 'sort'],
    page: '1',
    per_page: 10,
    sort: 'downloads',

    totalItems: computed.readOnly('model.meta.total'),

    categoryController: controller('category'),
    category: computed.alias('categoryController.model'),

    currentSortBy: computed('sort', function() {
        return (this.get('sort') === 'downloads') ? 'Downloads' : 'Alphabetical';
    }),
});
