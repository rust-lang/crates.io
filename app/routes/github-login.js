import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  beforeModel: function() {
    return ajax('/authorize_url').then(function(url) {
      window.location = url.url;
    });
  },
});
