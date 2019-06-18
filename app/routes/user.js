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

    model(params) {
        const { user_id } = params;
        return this.store.queryRecord('user', { user_id }).then(
            user => {
                params.user_id = user.get('id');
                return RSVP.hash({
                    crates: this.store.query('crate', params),
                    user,
                });
            },
            e => {
                if (e.errors.some(e => e.detail === 'Not Found')) {
                    this.flashMessages.queue(`User '${params.user_id}' does not exist`);
                    return this.replaceWith('index');
                }
            },
        );
    },

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
        controller.set('user', model.user);
        controller.set('allowFavorting', this.session.get('currentUser') !== model.user);

        if (controller.get('allowFavoriting')) {
            ajax(`/api/v1/users/${model.user.id}/favorited`)
                .then(d => controller.set('favorited', d.favorited))
                .finally(() => controller.set('fetchingFavorite', false));
        }
    },
});
