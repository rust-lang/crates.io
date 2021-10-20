import Controller from '@ember/controller';
import { action } from '@ember/object';
import { tracked } from '@glimmer/tracking';

import { reads } from 'macro-decorators';

import { pagination } from '../utils/pagination';

export default class CratesController extends Controller {
  queryParams = ['letter', 'page', 'per_page', 'sort'];
  @tracked letter = null;
  @tracked page = '1';
  @tracked per_page = 50;
  @tracked sort = 'alpha';
  alphabet = [...'ABCDEFGHIJKLMNOPQRSTUVWXYZ'];

  @reads('model.meta.total') totalItems;
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

  @action handleSelection(event) {
    this.set('letter', event.target.value);
  }
}
