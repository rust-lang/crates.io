import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

// TODO: reduce duplicatoin with controllers/crates

export default class MeCratesController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  page = '1';
  per_page = 10;
  sort = 'alpha';

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;

  @computed('sort')
  get currentSortBy() {
    if (this.sort === 'downloads') {
      return 'All-Time Downloads';
    } else if (this.sort === 'recent-downloads') {
      return 'Recent Downloads';
    } else if (this.sort === 'recent-updates') {
      return 'Recent Updates';
    } else if (this.sort === 'new') {
      return 'Newly Added';
    } else {
      return 'Alphabetical';
    }
  }
}
