import Component from '@ember/component';
import { empty } from '@ember/object/computed';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';

export default Component.extend({
    ajax: service(),
    flashMessages: service(),

    type: '',
    value: '',
    isEditing: false,
    user: null,
    disableSave: empty('user.email'),
    notValidEmail: false,
    prevEmail: '',
    emailIsNull: true,
    emailNotVerified: computed('user.email', 'user.email_verified', function() {
        let email = this.get('user.email');
        let verified = this.get('user.email_verified');

        if (email != null && !verified) {
            return true;
        } else {
            return false;
        }
    }),

    actions: {
        editEmail() {
            let email = this.get('value');
            let isEmailNull = function(email) {
                return (email == null);
            };

            this.set('emailIsNull', isEmailNull(email));
            this.set('isEditing', true);
            this.set('prevEmail', this.get('value'));
        },

        saveEmail() {
            let userEmail = this.get('value');
            let user = this.get('user');

            let emailIsProperFormat = function(userEmail) {
                let regExp = /^\S+@\S+\.\S+$/;
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
                        msg = 'An unknown error occurred while saving this email.';
                    }
                    this.set('serverError', msg);
                    this.get('flashMessages').queue(`Email error: ${err.errors[0].detail}`);
                    return this.replaceWith('me');
                });

            this.set('isEditing', false);
            this.set('notValidEmail', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
            this.set('value', this.get('prevEmail'));
        },

        resendEmail() {
            let userEmail = this.get('value');
            let user = this.get('user');

            this.get('ajax').raw(`/api/v1/users/${user.id}/resend`, { method: 'PUT',
                user: {
                    avatar: user.avatar,
                    email: user.email,
                    email_verified: user.email_verified,
                    kind: user.kind,
                    login: user.login,
                    name: user.name,
                    url: user.url
                }
            })
            .then(({response}) => {})
            .catch((error) => {
                if (error.payload) {
                    this.get('flashMessages').queue(`Error in email confirmation: ${error.payload.errors[0].detail}`)
                    console.log("error payload: " + error.payload.errors[0].detail);
                    return this.replaceWith('me');
                } else {
                    this.get('flashmessages').queue(`Unknown error in email confirmation`);
                    console.log("unknown error");
                    return this.replaceWith('me');
                }
            });
        }
    }
});
