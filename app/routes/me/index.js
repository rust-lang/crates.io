import Route from '@ember/routing/route';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
    model() {
        return {
            user: this.session.get('currentUser'),
            api_tokens: this.get('store').findAll('api-token'),
        };
    },
});
