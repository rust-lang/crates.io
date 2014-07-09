import Ember from 'ember';

export default Ember.Object.extend({
  savedTransition: null,

  init: function() {
    this.set('isLoggedIn', localStorage.isLoggedIn);
    this.set('currentUserId', localStorage.currentUserId);
    this.set('currentUser', null);
  },

  loginUser: function(user) {
    this.set('isLoggedIn', true);
    this.set('currentUser', user);
    this.set('currentUserId', user.id);
    localStorage.isLoggedIn = true;
    localStorage.currentUserId = user.id;
  },

  logoutUser: function() {
    this.set('savedTransition', null);
    this.set('isLoggedIn', null);
    this.set('currentUser', null);
    this.set('currentUserId', null);
    delete localStorage.isLoggedIn;
    delete localStorage.currentUserId;
  },
});
