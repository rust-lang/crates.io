import DS from 'ember-data';

export default DS.Model.extend({
    email: DS.attr('string'),
    email_verified: DS.attr('boolean'),
    email_verification_sent: DS.attr('boolean'),
    name: DS.attr('string'),
    login: DS.attr('string'),
    avatar: DS.attr('string'),
    url: DS.attr('string'),
    kind: DS.attr('string'),

    stats() {
        return this.store.adapterFor('user').stats(this.id);
    },

    favorite() {
        return this.store.adapterFor('user').favorite(this.get('id'));
    },

    unfavorite() {
        return this.store.adapterFor('user').unfavorite(this.get('id'));
    },

    favoriteUsers() {
        return this.store.adapterFor('user').favoriteUsers(this.get('id'));
    },
});
