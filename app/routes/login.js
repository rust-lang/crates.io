import Ember from 'ember';

export default Ember.Route.extend({
  activate: function() {
    Ember.$.getJSON('/authorize_url', function(url) {
      window.location = url;
    }).fail(function() {
      console.log('bad');
    });
  }
});
