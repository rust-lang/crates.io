import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',

  name: alias('ownedCrate.name'),
  controlId: computed('ownedCrate.id', function() {
    return `${this.ownedCrate.id}-email-notifications`;
  }),
  emailNotifications: alias('ownedCrate.email_notifications'),

  actions: {
    toggleEmailNotifications() {
      this.set('emailNotifications', !this.get('emailNotifications'));
    },
  },
});
