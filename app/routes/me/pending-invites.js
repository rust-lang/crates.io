import Ember from 'ember';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    model() {
        return this.get('store').findAll('crate-owner-invite');
    }
});
