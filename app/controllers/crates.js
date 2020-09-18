import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

import { pagination } from '../utils/pagination';

export default Controller.extend({
  queryParams: ['letter', 'page', 'per_page', 'sort'],
  letter: null,
  page: '1',
  per_page: 50,
  sort: 'alpha',
  alphabet: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(''),

  totalItems: readOnly('model.meta.total'),
  pagination: pagination(),

  currentSortBy: computed('sort', function () {
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
  }),

  actions: {
    handleSelection(event) {
      this.set('letter', event.target.value);
    },
  },
});
