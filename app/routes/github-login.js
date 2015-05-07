import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({
  beforeModel() {
    return ajax('/authorize_url').then((url) => {
      window.location = url.url;
    });
  },
});
