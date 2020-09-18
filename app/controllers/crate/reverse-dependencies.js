import Controller from '@ember/controller';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

export default Controller.extend({
  queryParams: ['page', 'per_page'],
  page: '1',
  per_page: 10,
  crate: null,

  totalItems: readOnly('model.meta.total'),
  pagination: pagination(),
});
