import Ember from 'ember';

export default Ember.Component.extend({
    type: '',
    value: '',
    isEditing: false,
    user: null,

    actions: {
        editEmail() {
            this.set('isEditing', true);
        },

        saveEmail() {
            var userEmail = this.get('value');

            var user = this.get('user');
            user.set('email', userEmail);
            user.save();

            console.log('username: ' + user.get('name'));
            console.log('userEmail: ' + userEmail);
            this.set('isEditing', false);
        },

        cancelEdit() {
            this.set('isEditing', false);
        }
    }
});
