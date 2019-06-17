import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
    model() {
        return this.store.findAll('crate-owner-invite');
    },
});
