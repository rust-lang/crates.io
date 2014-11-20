import Ember from 'ember';

export default Ember.Object.extend({
    savedTransition: null,
    isLoggedIn: false,
    currentUser: null,

    init: function() {
        this.set('isLoggedIn', localStorage.getItem('isLoggedIn') === '1');
        this.set('currentUser', null);
    },

    loginUser: function(user) {
        this.set('isLoggedIn', true);
        this.set('currentUser', user);
        localStorage.setItem('isLoggedIn', '1');
    },

    logoutUser: function() {
        this.set('savedTransition', null);
        this.set('isLoggedIn', null);
        this.set('currentUser', null);
        localStorage.removeItem('isLoggedIn');
    },
});
