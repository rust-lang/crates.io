import Ember from 'ember';

export function initialize() {
    Ember.$.getScript('https://www.google.com/jsapi', function() {
        window.google.load('visualization', '1.0', {
            packages: ['corechart'],
            callback() {
                window.googleChartsLoaded = true;
                Ember.$(document).trigger('googleChartsLoaded');
            }
        });
    });
}

export default {
    name: 'google',
    initialize
};
