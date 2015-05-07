import Ember from 'ember';

export default Ember.Route.extend({
    activate() {
      Ember.$.getJSON('/logout', () => {
        Ember.run(() => {
          this.session.logoutUser();
          this.transitionTo('index');
        });
      });
    }
});
