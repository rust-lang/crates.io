import { readOnly } from '@ember/object/computed';
import Controller from '@ember/controller';
import { computed } from '@ember/object';

import PaginationMixin from '../mixins/pagination';

export default Controller.extend(PaginationMixin, {
  queryParams: ['page', 'per_page', 'sort'],
  page: '1',
  per_page: 10,
  sort: 'alpha',

  totalItems: readOnly('model.crates.meta.total'),

  currentSortBy: computed('sort', function() {
    if (this.sort === 'downloads') {
      return 'All-Time Downloads';
    } else if (this.sort === 'recent-downloads') {
      return 'Recent Downloads';
    } else if (this.get('sort') === 'recent-updates') {
      return 'Recent Updates';
    } else {
      return 'Alphabetical';
    }
  }),
});
