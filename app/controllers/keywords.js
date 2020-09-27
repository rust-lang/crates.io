import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../utils/pagination';

export default class KeywordsController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  page = '1';
  per_page = 10;
  sort = 'crates';

  @readOnly('model.meta.total') totalItems;

  @pagination() pagination;

  @computed('sort')
  get currentSortBy() {
    return this.sort === 'crates' ? '# Crates' : 'Alphabetical';
  }
}
