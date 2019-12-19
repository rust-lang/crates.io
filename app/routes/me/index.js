import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
  actions: {
    willTransition: function() {
      this.controller
        .setProperties({
          emailNotificationsSuccess: false,
          emailNotificationsError: false,
        })
        .clear();
    },
  },
  model() {
    return {
      user: this.get('session.currentUser'),
      ownedCrates: this.get('session.ownedCrates'),
      api_tokens: this.store.findAll('api-token'),
    };
  },
});
