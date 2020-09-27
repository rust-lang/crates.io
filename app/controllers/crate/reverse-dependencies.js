import Controller from '@ember/controller';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

export default class ReverseDependenciesController extends Controller {
  queryParams = ['page', 'per_page'];
  page = '1';
  per_page = 10;
  crate = null;

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;
}
