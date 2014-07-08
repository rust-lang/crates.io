import Ember from 'ember';

export default Ember.Object.extend({
  savedTransition: null,

  init: function() {
    this.set('isLoggedIn', localStorage.isLoggedIn);
  },

  loginUser: function() {
    this.set('isLoggedIn', true);
    localStorage.isLoggedIn = true;
  },

  logoutUser: function() {
    this.set('savedTransition', null);
    this.set('isLoggedIn', null);
    delete localStorage.isLoggedIn;
  },
});
