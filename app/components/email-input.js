import Component from '@ember/component';
import { empty } from '@ember/object/computed';
import { computed } from '@ember/object';
import { inject as service } from '@ember/service';
import ajax from 'ember-fetch/ajax';

export default Component.extend({
  tagName: '',
  flashMessages: service(),
  type: '',
  value: '',
  isEditing: false,
  user: null,
  disableSave: empty('user.email'),
  notValidEmail: false,
  prevEmail: '',

  emailIsNull: computed('user.email', function() {
    let email = this.get('user.email');
    return email == null;
  }),

  emailNotVerified: computed('user.{email,email_verified}', function() {
    let email = this.get('user.email');
    let verified = this.get('user.email_verified');

    return email != null && !verified;
  }),

  isError: false,
  emailError: '',
  disableResend: false,

  resendButtonText: computed('disableResend', 'user.email_verification_sent', function() {
    if (this.disableResend) {
      return 'Sent!';
    } else if (this.get('user.email_verification_sent')) {
      return 'Resend';
    } else {
      return 'Send verification email';
    }
  }),

  actions: {
    editEmail() {
      let email = this.value;
      let isEmailNull = function(email) {
        return email == null;
      };

      this.set('emailIsNull', isEmailNull(email));
      this.set('isEditing', true);
      this.set('prevEmail', this.value);
    },

    saveEmail() {
      let userEmail = this.value;
      let user = this.user;

      let emailIsProperFormat = function(userEmail) {
        let regExp = /^\S+@\S+\.\S+$/;
        return regExp.test(userEmail);
      };

      if (!emailIsProperFormat(userEmail)) {
        this.set('notValidEmail', true);
        return;
      }

      user.set('email', userEmail);
      user
        .save()
        .then(() => {
          this.set('serverError', null);
          this.set('user.email_verification_sent', true);
          this.set('user.email_verified', false);
        })
        .catch(err => {
          let msg;
          if (err.errors && err.errors[0] && err.errors[0].detail) {
            msg = `An error occurred while saving this email, ${err.errors[0].detail}`;
          } else {
            msg = 'An unknown error occurred while saving this email.';
          }
          this.set('serverError', msg);
          this.set('isError', true);
          this.set('emailError', `Error in saving email: ${msg}`);
        });

      this.set('isEditing', false);
      this.set('notValidEmail', false);
      this.set('disableResend', false);
    },

    cancelEdit() {
      this.set('isEditing', false);
      this.set('value', this.prevEmail);
    },

    async resendEmail() {
      let user = this.user;

      try {
        await ajax(`/api/v1/users/${user.id}/resend`, { method: 'PUT' });
        this.set('disableResend', true);
      } catch (error) {
        if (error.errors) {
          this.set('isError', true);
          this.set('emailError', `Error in resending message: ${error.errors[0].detail}`);
        } else {
          this.set('isError', true);
          this.set('emailError', 'Unknown error in resending message');
        }
      }
    },
  },
});
