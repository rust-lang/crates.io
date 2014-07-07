import Ember from 'ember';

export default Ember.Controller.extend({
  isLoggedIn: localStorage.isLoggedIn,
  savedTransition: null,

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

