import Controller from '@ember/controller';
import { readOnly } from '@ember/object/computed';

import PaginationMixin from '../../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page'],
    page: '1',
    per_page: 10,
    crate: null,

    totalItems: readOnly('model.meta.total'),
});
