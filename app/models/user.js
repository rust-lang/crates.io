import DS from 'ember-data';

export default DS.Model.extend({
    email: DS.attr('string'),
    name: DS.attr('string'),
    login: DS.attr('string'),
    api_token: DS.attr('string'),
    avatar: DS.attr('string'),

    smallAvatar: function() {
        return this.get('avatar') + '&s=22';
    }.property('avatar'),

    mediumSmallAvatar: function() {
        return this.get('avatar') + '&s=32';
    }.property('avatar'),

    mediumAvatar: function() {
        return this.get('avatar') + '&s=85';
    }.property('avatar'),
});
