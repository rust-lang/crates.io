import DS from 'ember-data';
import Ember from 'ember';

export default DS.Model.extend({
    email: DS.attr('string'),
    name: DS.attr('string'),
    login: DS.attr('string'),
    api_token: DS.attr('string'),
    avatar: DS.attr('string'),
    url: DS.attr('string'),
    kind: DS.attr('string'),
    org_name: Ember.computed('login', function() {
        let login = this.get('login');
        let login_split = login.split(':');
        return login_split[1];
    })
});
