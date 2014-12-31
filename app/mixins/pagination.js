import Ember from 'ember';

var VIEWABLE_PAGES = 9;

export default Ember.Mixin.create({

    // Gives page numbers to the surrounding 9 pages.
    pages: function() {
        var pages = [];
        var currentPage = this.get('currentPage');
        var availablePages = this.get('availablePages');
        var lowerBound = 0;
        var upperBound = 0;

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
        for (var i = lowerBound; i < upperBound; i++) {
            pages.push(i + 1);
        }
        return pages;
    }.property('currentPage', 'availablePages'),

    currentPage: function() {
        return parseInt(this.get('selectedPage'), 10) || 1;
    }.property('selectedPage'),

    currentPageStart: function() {
        if (this.get('totalItems') === 0) { return 0; }
        return (this.get('currentPage') - 1) * this.get('itemsPerPage') + 1;
    }.property('currentPage', 'itemsPerPage', 'totalItems'),

    currentPageEnd: function() {
        return Math.min(this.get('currentPage') * this.get('itemsPerPage'),
                        this.get('totalItems'));
    }.property('currentPage', 'itemsPerPage', 'totalItems'),

    nextPage: function() {
        var nextPage = this.get('currentPage') + 1;
        var availablePages = this.get('availablePages');
        if (nextPage <= availablePages) {
            return nextPage;
        } else {
            return this.get('currentPage');
        }
    }.property('currentPage', 'availablePages'),

    prevPage: function() {
        var prevPage = this.get('currentPage') - 1;
        if (prevPage > 0) {
            return prevPage;
        } else {
            return this.get('currentPage');
        }

    }.property('currentPage'),

    availablePages: function() {
        return Math.ceil((this.get('totalItems') /
                          this.get('itemsPerPage')) || 1);
    }.property('totalItems', 'itemsPerPage'),

    // wire up these ember-style variables to the expected query parameters
    itemsPerPage: function() {
        return this.get('per_page');
    }.property('per_page'),

    selectedPage: function() { return this.get('page'); }.property('page'),
});
