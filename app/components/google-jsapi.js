import Ember from 'ember';

export default Ember.Component.extend({
    tagName: '',

    didInsertElement() {
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
});
