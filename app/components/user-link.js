import Ember from 'ember';

export default Ember.Component.extend({
  user: null,
  attributeBindings: ['title', 'href'],
  tagName: 'a',

  title: function() {
      return this.get('user.login');
  }.property('user'),

  'href': function() {
      // TODO replace this with a link to a native crates.io profile
      // page when they exist.
      return 'https://github.com/' + this.get('user.login');
  }.property('user'),
});
