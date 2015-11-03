import Ember from 'ember';

const { computed } = Ember;

export default Ember.Component.extend({
  user: null,
  attributeBindings: ['title', 'href'],
  tagName: 'a',

  title: computed.readOnly('user.login'),
  href: computed('user', function() {
      // TODO replace this with a link to a native crates.io profile
      // page when they exist.
      return this.get('user.url');
  })
});
