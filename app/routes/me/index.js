import Ember from 'ember';
import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    model() {
        return {
            user: this.session.get('currentUser'),
            api_tokens: this.get('store').findAll('api-token'),
        };
    },
});
