import Component from '@ember/component';
import $ from 'jquery';

export default Component.extend({
    tagName: '',

    didInsertElement() {
        $.getScript('https://www.google.com/jsapi', function() {
            window.google.load('visualization', '1.0', {
                packages: ['corechart'],
                callback() {
                    window.googleChartsLoaded = true;
                    $(document).trigger('googleChartsLoaded');
                }
            });
        });
    }
});
