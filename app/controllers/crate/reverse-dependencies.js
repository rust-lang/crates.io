import Controller, { inject as controller } from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../../mixins/pagination';

export default Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page'],
    page: '1',
    per_page: 10,

    crateController: controller('crate'),
    crate: computed.alias('crateController.model'),

    totalItems: computed.readOnly('model.meta.total'),
});
