import Ember from 'ember';

export default Ember.Object.extend({
  savedTransition: null,

  init: function() {
    this.set('isLoggedIn', localStorage.isLoggedIn);
    var json;
    try {
      json = JSON.parse(localStorage.currentUser);
    } catch (e) {
      this.set('currentUser', null);
      return;
    }
    var store = this.container.lookup('store:main');
    this.set('currentUser', store.push('user', json));
  },

  loginUser: function(user) {
    this.set('isLoggedIn', true);
    this.set('currentUser', user);
    localStorage.isLoggedIn = true;
    localStorage.currentUser = JSON.stringify(user);
  },

  logoutUser: function() {
    this.set('savedTransition', null);
    this.set('isLoggedIn', null);
    this.set('currentUser', null);
    delete localStorage.isLoggedIn;
    delete localStorage.currentUser;
  },
});
