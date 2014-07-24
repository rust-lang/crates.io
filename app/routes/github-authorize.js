import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  beforeModel: function(transition) {
    return ajax('/authorize', {data: transition.queryParams}).then(function(d) {
      localStorage.github_response = JSON.stringify({ ok: true, data: d });
    }).catch(function(d) {
      localStorage.github_response = JSON.stringify({ ok: false, data: d });
    }).finally(function() {
      window.close();
    });
  },
});
