import Component from '@ember/component';
import trigger from '../utils/trigger';

export default Component.extend({
    tagName: '',

    didInsertElement() {
        getScript('https://www.google.com/jsapi', () => {
            window.google.load('visualization', '1.0', {
                packages: ['corechart'],
                callback() {
                    window.googleChartsLoaded = true;
                    trigger(document, 'googleChartsLoaded');
                }
            });
        });
    }
});
function getScript(source, callback) {
    let script = document.createElement('script');
    let [prior] = document.getElementsByTagName('script');
    script.async = 1;

    script.onload = script.onreadystatechange = function(_, isAbort) {
        if (isAbort || !script.readyState || /loaded|complete/.test(script.readyState)) {
            script.onload = script.onreadystatechange = null;
            script = undefined;

            if (!isAbort) {
                if (callback) {
                    callback();
                }
            }
        }
    };

    script.src = source;
    prior.parentNode.insertBefore(script, prior);
}
