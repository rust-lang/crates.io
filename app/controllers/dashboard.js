import Ember from 'ember';
import ajax from 'ic-ajax';

var TO_SHOW = 5;

export default Ember.ObjectController.extend({
    fetchingCrates: true,
    fetchingFollowing: true,
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
                // Wow, I sure wish I knew why none of this works like it should!
                //
                // TODO: fix this so we don't have to push everything in and
                //       then re-find the things
                self.store.pushMany('crate', data.crates);
                var versions = self.store.pushMany('version', data.versions);
                versions.forEach(function(v) {
                    self.store.find('crate', v._data.crate_id).then(function(c) {
                        v.set('crate', c);
                    });
                });
                self.get('myFeed').pushObjects(versions);
                self.set('hasMore', data.meta.more);
            }).finally(function() {
                self.set('loadingMore', false);
            });
        },
    },
});
