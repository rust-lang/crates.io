import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { pagination } from '../utils/pagination';

const MAX_PAGES = 50;

export default class TeamController extends Controller {
  queryParams = ['page', 'per_page', 'sort'];
  @tracked page = '1';
  @tracked per_page = 10;
  @tracked sort = 'alpha';

  @pagination({ maxPages: MAX_PAGES }) pagination;

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
    return this.model.crates.meta.total ?? 0;
  }
}
