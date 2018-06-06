import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
    model() {
        return {
            user: this.get('session.currentUser'),
            api_tokens: this.store.findAll('api-token'),
        };
    },
});
