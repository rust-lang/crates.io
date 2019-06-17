import Controller from '@ember/controller';
import { sort, filterBy, notEmpty } from '@ember/object/computed';
import { inject as service } from '@ember/service';

export default Controller.extend({
    // eslint-disable-next-line ember/avoid-leaking-state-in-ember-objects
    tokenSort: ['created_at:desc'],

    sortedTokens: sort('model.api_tokens', 'tokenSort'),

    flashMessages: service(),

    isResetting: false,

    newTokens: filterBy('model.api_tokens', 'isNew', true),
    disableCreate: notEmpty('newTokens'),

    actions: {
        startNewToken() {
            this.store.createRecord('api-token', {
                created_at: new Date(Date.now() + 2000),
            });
        },
    },
});
