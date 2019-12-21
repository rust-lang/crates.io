import { readOnly } from '@ember/object/computed';
import Mixin from '@ember/object/mixin';
import { computed } from '@ember/object';

const VIEWABLE_PAGES = 9;

// eslint-disable-next-line ember/no-new-mixins
export default Mixin.create({
  // Gives page numbers to the surrounding 9 pages.
  pages: computed('currentPage', 'availablePages', function() {
    let pages = [];
    let currentPage = this.currentPage;
    let availablePages = this.availablePages;
    let lowerBound = 0;
    let upperBound = 0;

    // Always show the same number of pages even if we're
    // at the beginning or at the end of the list.
    if (availablePages - currentPage < Math.ceil(VIEWABLE_PAGES / 2)) {
      lowerBound = Math.max(0, availablePages - VIEWABLE_PAGES);
      upperBound = availablePages;
    } else if (currentPage <= Math.ceil(VIEWABLE_PAGES / 2)) {
      lowerBound = 0;
      upperBound = Math.min(availablePages, VIEWABLE_PAGES);
    } else {
      lowerBound = currentPage - Math.ceil(VIEWABLE_PAGES / 2);
      upperBound = currentPage + Math.floor(VIEWABLE_PAGES / 2);
    }
    for (let i = lowerBound; i < upperBound; i++) {
      pages.push(i + 1);
    }
    return pages;
  }),

  currentPage: computed('selectedPage', function() {
    return parseInt(this.selectedPage, 10) || 1;
  }),

  currentPageStart: computed('currentPage', 'itemsPerPage', 'totalItems', function() {
    if (this.totalItems === 0) {
      return 0;
    }
    return (this.currentPage - 1) * this.itemsPerPage + 1;
  }),

  currentPageEnd: computed('currentPage', 'itemsPerPage', 'totalItems', function() {
    return Math.min(this.currentPage * this.itemsPerPage, this.totalItems);
  }),

  nextPage: computed('currentPage', 'availablePages', function() {
    let nextPage = this.currentPage + 1;
    let availablePages = this.availablePages;
    if (nextPage <= availablePages) {
      return nextPage;
    } else {
      return this.currentPage;
    }
  }),

  prevPage: computed('currentPage', function() {
    let prevPage = this.currentPage - 1;
    if (prevPage > 0) {
      return prevPage;
    } else {
      return this.currentPage;
    }
  }),

  availablePages: computed('totalItems', 'itemsPerPage', function() {
    return Math.ceil(this.totalItems / this.itemsPerPage || 1);
  }),

  // wire up these ember-style variables to the expected query parameters
  itemsPerPage: readOnly('per_page'),
  selectedPage: readOnly('page'),
});
