import Ember from 'ember';

var TO_SHOW = 5;

export default Ember.ObjectController.extend({
    fetchingCrates: true,
    fetchingFollowing: true,
    myCrates: [],
    myFollowing: [],

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
});
