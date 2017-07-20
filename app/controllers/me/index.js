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

    isEditing: false,

    actions: {
        startNewToken() {
            this.get('store').createRecord('api-token', {
                created_at: new Date(Date.now() + 2000),
            });
        },

        editEmail() {
            this.set('isEditing', true);
        },

        saveEmail() {
            var userEmail = this.get('userEmail');

            var user = this.session.currentUser;
            user.get('email');
            user.set('email', userEmail);
            user.save();

            this.set('isEditing', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
        }
    }
});
