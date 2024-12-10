import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { pagination } from '../../utils/pagination';

// TODO: reduce duplicatoin with controllers/crates

export default class MeCratesController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  @tracked page = '1';
  @tracked per_page = 10;
  @tracked sort = 'alpha';

  @pagination() pagination;

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

  get totalItems() {
    return this.model.meta.total ?? 0;
  }
}
