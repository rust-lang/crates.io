import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
    beforeModel(transition) {
        return ajax('/authorize', {data: transition.queryParams}).then((d) => {
            var item = JSON.stringify({ ok: true, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).catch((d) => {
            var item = JSON.stringify({ ok: false, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).finally(() => {
            window.close();
        });
    },
});
