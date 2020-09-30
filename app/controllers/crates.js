import Controller from '@ember/controller';
import { action, computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../utils/pagination';

export default class CratesController extends Controller {
  queryParams = ['letter', 'page', 'per_page', 'sort'];
  letter = null;
  page = '1';
  per_page = 50;
  sort = 'alpha';
  alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');

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

  @action handleSelection(event) {
    this.set('letter', event.target.value);
  }
}
