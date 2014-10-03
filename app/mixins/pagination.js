import Ember from 'ember';

export default Ember.Mixin.create({
    pages: function() {
        var availablePages = this.get('availablePages');
        var pages = [];
        for (var i = 0; i < availablePages; i++) {
            pages.push(i + 1);
        }
        return pages;
    }.property('availablePages'),

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
