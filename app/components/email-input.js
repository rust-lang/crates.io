import Component from '@ember/component';
import { empty } from '@ember/object/computed';

export default Component.extend({
    type: '',
    value: '',
    isEditing: false,
    user: null,
    disableSave: empty('user.email'),

    actions: {
        editEmail() {
            this.set('isEditing', true);
        },

        saveEmail() {
            var userEmail = this.get('value');
            var user = this.get('user');

            user.set('email', userEmail);
            user.save();

            this.set('isEditing', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
        }
    }
});
