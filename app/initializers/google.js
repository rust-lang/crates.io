import Ember from 'ember';

export var initialize = function() {
    Ember.$.getScript('https://www.google.com/jsapi', function() {
        window.google.load('visualization', '1.0', {
            'packages': ['corechart'],
            'callback': function() {
                window.googleChartsLoaded = true;
                Ember.$(document).trigger('googleChartsLoaded');
            }
        });
    });
};

export default {
    name: 'google',

    initialize: initialize
};
