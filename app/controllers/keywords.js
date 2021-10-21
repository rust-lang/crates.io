import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { reads } from 'macro-decorators';

import { pagination } from '../utils/pagination';

export default class KeywordsController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  @tracked page = '1';
  @tracked per_page = 10;
  @tracked sort = 'crates';

  @reads('model.meta.total') totalItems;

  @pagination() pagination;

  get currentSortBy() {
    return this.sort === 'crates' ? '# Crates' : 'Alphabetical';
  }
}
