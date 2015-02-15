import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
    beforeModel: function(transition) {
        return ajax('/authorize', {data: transition.queryParams}).then(function(d) {
            var item = JSON.stringify({ ok: true, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).catch(function(d) {
            var item = JSON.stringify({ ok: false, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).finally(function() {
            window.close();
        });
    },
});
