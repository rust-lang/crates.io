<<<<<<< 120f8008fb08c21fc6cd239c1100692a0ff487e6
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import RSVP from 'rsvp';
import ajax from 'ic-ajax';

export default Route.extend({
    flashMessages: service(),

    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    data: {},

    model(params) {
        const { user_id } = params;
        return this.store.queryRecord('user', { user_id }).then(
            (user) => {
                params.user_id = user.get('id');
                return RSVP.hash({
                    crates: this.store.query('crate', params),
                    user
                });
            },
            (e) => {
                if (e.errors.any(e => e.detail === 'Not Found')) {
                    this.get('flashMessages').queue(`User '${params.user_id}' does not exist`);
                    return this.replaceWith('index');
                }
            }
        );
    },

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
        controller.set('user', model.user);
        controller.set(
            'allowFavorting',
            this.session.get('currentUser') !== model.user
        );
        
        if (controller.get('allowFavorting')) {
            ajax(`/api/v1/users/${model.user.id}/favorited`)
                .then((d) => controller.set('favorited', d.favorited))
                .finally(() => controller.set('fetchingFavorite', false));
        }
    },
});
