import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

// TODO: reduce duplicatoin with controllers/me/crates

export default Controller.extend({
  queryParams: ['page', 'per_page', 'sort'],
  page: '1',
  per_page: 10,
  sort: 'alpha',

  totalItems: readOnly('model.meta.total'),
  pagination: pagination(),

  currentSortBy: computed('sort', function () {
    return this.sort === 'downloads' ? 'Downloads' : 'Alphabetical';
  }),
});
