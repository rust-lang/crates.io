import Ember from 'ember';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    model: function() {
        return this.session.get('currentUser');
    },
});
