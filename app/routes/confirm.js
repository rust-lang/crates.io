import Ember from 'ember';
import { inject as service } from '@ember/service';

export default Ember.Route.extend({
    flashMessages: service(),
    ajax: service(),

    model(params) {
        return this.get('ajax').raw(`/api/v1/confirm/${params.email_token}`, { method: 'PUT', data: {}})
            .then(({response}) => {})
            .catch((error) => {
                if (error.payload) {
                    this.get('flashMessages').queue(`Error in email confirmation: ${error.payload.errors[0].detail}`);
                    return this.replaceWith('index');
                } else {
                    this.get('flashMessages').queue(`Unknown error in email confirmation`);
                    return this.replaceWith('index');
                }
            });
    }
});
