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

    actions: {
        editEmail() {
            this.set('isEditing', true);
            this.set('prevEmail', this.get('value'));
        },

        saveEmail() {
            var userEmail = this.get('value');
            var user = this.get('user');

            var emailIsProperFormat = function(userEmail) {
                var regExp = /\S+@\S+\.\S+/;
                return egExp.test(userEmail);
            };

            if (!emailIsProperFormat(userEmail)) {
                this.set('notValidEmail', true);
                return;
            }

            user.set('email', userEmail);
            user.save();

            this.set('isEditing', false);
            this.set('notValidEmail', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
            this.set('value', this.get('prevEmail'));
        }
    }
});
