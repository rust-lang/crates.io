import Ember from 'ember';
import ajax from 'ic-ajax';

var TO_SHOW = 5;

export default Ember.ObjectController.extend({
    fetchingFeed: true,
    loadingMore: false,
    hasMore: false,
    myCrates: [],
    myFollowing: [],
    myFeed: [],

    visibleCrates: function() {
        return this.get('myCrates').slice(0, TO_SHOW);
    }.property('myCrates'),

    visibleFollowing: function() {
        return this.get('myFollowing').slice(0, TO_SHOW);
    }.property('myFollowing'),

    hasMoreCrates: function() {
        return this.get('myCrates').length > TO_SHOW;
    }.property('myCrates'),

    hasMoreFollowing: function() {
        return this.get('myFollowing').length > TO_SHOW;
    }.property('myFollowing'),

    actions: {
        loadMore: function() {
            var self = this;
            this.set('loadingMore', true);
            var page = (this.get('myFeed').length / 10) + 1;
            ajax('/me/updates?page=' + page).then(function(data) {
                self.store.pushMany('crate', data.crates);
                var versions = self.store.pushMany('version', data.versions);
                self.get('myFeed').pushObjects(versions);
                self.set('hasMore', data.meta.more);
            }).finally(function() {
                self.set('loadingMore', false);
            });
        },
    },
});
