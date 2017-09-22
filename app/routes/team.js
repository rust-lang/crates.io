import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import RSVP from 'rsvp';

export default Route.extend({
    flashMessages: service(),

    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },
    data: {},

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
    },

    model(params) {
        const { team_id } = params;

        return this.store.queryRecord('team', { team_id }).then(
            (team) => {
                params.team_id = team.get('id');
                return RSVP.hash({
                    crates: this.store.query('crate', params),
                    team
                });
            },
            (e) => {
                if (e.errors.some(e => e.detail === 'Not Found')) {
                    this.get('flashMessages').queue(`Team '${params.team_id}' does not exist`);
                    return this.replaceWith('index');
                }
            }
        );
    },
});
