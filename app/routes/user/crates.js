import Ember from 'ember';
import AuthenticatedRoute from '../../mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        params.user_id = this.paramsFor("user").user_id;
        return Ember.RSVP.hash({
            user: this.store.find('user', params.user_id),
            crates: this.store.query('crate', params)
        }).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('nextFlashError', `User '${params.user_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },
});

