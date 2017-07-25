import Component from '@ember/component';
import { empty } from '@ember/object/computed';

export default Component.extend({
    type: '',
    value: '',
    isEditing: false,
    user: null,
    disableSave: empty('user.email'),
    notValidEmail: false,
    prevEmail: '',
    emailIsNull: true,

    actions: {
        editEmail() {
            let user = this.get('user');
            let isEmailNull = function(user) {
                if (user.email == null) {
                    return true;
                } else {
                    return false;
                }
            };

            this.set('emailIsNull', isEmailNull(user));
            this.set('isEditing', true);
            this.set('prevEmail', this.get('value'));
        },

        saveEmail() {
            let userEmail = this.get('value');
            let user = this.get('user');

            let emailIsProperFormat = function(userEmail) {
                let regExp = /\S+@\S+\.\S+/;
                return regExp.test(userEmail);
            };

            if (!emailIsProperFormat(userEmail)) {
                this.set('notValidEmail', true);
                return;
            }

            user.set('email', userEmail);
            user.save()
                .then(() => this.set('serverError', null))
                .catch(err => {
                    let msg;
                    if (err.errors && err.errors[0] && err.errors[0].detail) {
                        msg = `An error occurred while saving this email, ${err.errors[0].detail}`;
                    } else {
                        msg = 'An unknown error occurred while saving this token.';
                    }
                    this.set('serverError', msg);
                });

            this.set('isEditing', false);
            this.set('notValidEmail', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
            this.set('value', this.get('prevEmail'));
        }
    }
});
