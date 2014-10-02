import DS from 'ember-data';

export default DS.Model.extend({
    email: DS.attr('string'),
    name: DS.attr('name'),
    login: DS.attr('login'),
    api_token: DS.attr('string'),
    avatar: DS.attr('string'),

    smallAvatar: function() {
        return this.get('avatar') + '?s=22';
    }.property('avatar'),
});
