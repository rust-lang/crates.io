import Component from '@ember/component';
import { empty, or } from '@ember/object/computed';

export default Component.extend({
    emptyName: empty('api_token.name'),
    disableCreate: or('api_token.isSaving', 'emptyName'),
    serverError: null,

    didInsertElement() {
        if (this.get('api_token.isNew')) {
            this.$('input').focus();
        }
    },

    actions: {
        saveToken() {
            this.get('api_token')
                .save()
                .then(() => this.set('serverError', null))
                .catch(err => {
                    let msg;
                    if (err.errors && err.errors[0] && err.errors[0].detail) {
                        msg = `An error occurred while saving this token, ${err.errors[0].detail}`;
                    } else {
                        msg = 'An unknown error occurred while saving this token';
                    }
                    this.set('serverError', msg);
                });
        },
        revokeToken() {
            this.get('api_token')
                .destroyRecord()
                .catch(err => {
                    let msg;
                    if (err.errors && err.errors[0] && err.errors[0].detail) {
                        msg = `An error occurred while revoking this token, ${err.errors[0].detail}`;
                    } else {
                        msg = 'An unknown error occurred while revoking this token';
                    }
                    this.set('serverError', msg);
                });
        },
    }
});
