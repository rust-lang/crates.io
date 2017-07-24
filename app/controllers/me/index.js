import Controller from '@ember/controller';
import { sort, filterBy, notEmpty } from '@ember/object/computed';
import { inject as service } from '@ember/service';

export default Controller.extend({
    tokenSort: ['created_at:desc'],

    sortedTokens: sort('model.api_tokens', 'tokenSort'),

    ajax: service(),

    flashMessages: service(),

    isResetting: false,

    newTokens: filterBy('model.api_tokens', 'isNew', true),
    disableCreate: notEmpty('newTokens'),

    actions: {
        startNewToken() {
            this.get('store').createRecord('api-token', {
                created_at: new Date(Date.now() + 2000),
            });
        },
    }
});
