import Component from '@ember/component';
import { inject as service } from '@ember/service';
import { computed } from '@ember/object';
import { notEmpty } from '@ember/object/computed';

export default Component.extend({
  session: service(),

  text: computed('session.currentUser.{email_verified,has_tokens}', function() {
    const user = this.get('session.currentUser');
    if (!user || (user.email_verified && user.has_tokens)) return '';

    const textArray = [
      !user.email_verified && 'verify your email address',
      !user.email_verified && !user.has_tokens && ' and ',
      !user.has_tokens && 'create an API token',
      '!',
    ].filter(e => !!e);

    return textArray.join('');
  }),

  showMessage: notEmpty('text').readOnly(),

  tagName: '',
});
