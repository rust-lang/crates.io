import Ember from 'ember';

export default Ember.Object.extend({
    savedTransition: null,
    isLoggedIn: false,
    currentUser: null,

    init: function() {
        var isLoggedIn;
        try {
            isLoggedIn = localStorage.getItem('isLoggedIn') === '1';
        } catch (e) {
            isLoggedIn = false;
        }
        this.set('isLoggedIn', isLoggedIn);
        this.set('currentUser', null);
    },

    loginUser: function(user) {
        this.set('isLoggedIn', true);
        this.set('currentUser', user);
        try {
            localStorage.setItem('isLoggedIn', '1');
        } catch (e) {}
    },

    logoutUser: function() {
        this.set('savedTransition', null);
        this.set('isLoggedIn', null);
        this.set('currentUser', null);
        try {
            localStorage.removeItem('isLoggedIn');
        } catch (e) {}
    },
});
