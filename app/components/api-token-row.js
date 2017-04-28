import Ember from 'ember';

export default Ember.Component.extend({
    emptyName: Ember.computed.empty('api_token.name'),
    disableCreate: Ember.computed.or('api_token.isSaving', 'emptyName'),
    serverError: null,

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
