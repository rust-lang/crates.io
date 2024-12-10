import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { pagination } from '../utils/pagination';

export default class KeywordIndexController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  @tracked page = '1';
  @tracked per_page = 10;
  @tracked sort = 'recent-downloads';

  @pagination() pagination;

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

  get totalItems() {
    return this.model.crates.meta.total ?? 0;
  }
}
