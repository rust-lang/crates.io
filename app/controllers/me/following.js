import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

// TODO: reduce duplicatoin with controllers/me/crates

export default class FollowingController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  page = '1';
  per_page = 10;
  sort = 'alpha';

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;

  @computed('sort')
  get currentSortBy() {
    return this.sort === 'downloads' ? 'Downloads' : 'Alphabetical';
  }
}
