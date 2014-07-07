import Ember from 'ember';

export default Ember.Route.extend({
  activate: function() {
    var self = this;
    Ember.$.getJSON('/logout', function() {
      self.controllerFor('application').logoutUser();
      self.transitionTo('index');
    }).fail(function() {
      console.log('bad');
    });
  }
});
