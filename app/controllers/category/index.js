import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../../utils/pagination';

export default class CategoryIndexController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  page = '1';
  per_page = 10;
  sort = 'recent-downloads';

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;

  category = null;

  @computed('sort')
  get currentSortBy() {
    if (this.sort === 'downloads') {
      return 'All-Time Downloads';
    } else if (this.sort === 'alpha') {
      return 'Alphabetical';
    } else if (this.sort === 'new') {
      return 'Newly Added';
    } else if (this.sort === 'recent-updates') {
      return 'Recent Updates';
    } else {
      return 'Recent Downloads';
    }
  }
}
