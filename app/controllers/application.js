import Ember from 'ember';

export default Ember.Controller.extend({
  isLoggedIn: localStorage.isLoggedIn,
  savedTransition: null,
  flashError: null,
  nextFlashError: null,

  loginUser: function() {
    this.set('isLoggedIn', true);
    localStorage.isLoggedIn = true;
  },

  logoutUser: function() {
    this.set('savedTransition', null);
    this.set('isLoggedIn', null);
    delete localStorage.isLoggedIn;
  },

  stepFlash: function() {
    this.set('flashError', this.get('nextFlashError'));
    this.set('nextFlashError', null);
  },

  setFlashError: function(s) {
    this.set('nextFlashError', s);
  },
});

