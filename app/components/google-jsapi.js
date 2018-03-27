import Component from '@ember/component';
import ajax from 'ember-fetch/ajax';
import trigger from '../utils/trigger';

export default Component.extend({
    tagName: '',

    didInsertElement() {
        ajax('https://www.google.com/jsapi', () => {
            window.google.load('visualization', '1.0', {
                packages: ['corechart'],
                callback() {
                    window.googleChartsLoaded = true;
                    trigger('googleChartsLoaded');
                }
            });
        });
    }
});
