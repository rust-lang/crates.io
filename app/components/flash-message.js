import Component from '@ember/component';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

export default Component.extend({
  flashMessages: service(),
  message: readOnly('flashMessages.message'),
  options: readOnly('flashMessages.options'),
  type: computed('flashMessages.options', function () {
    return this.get('flashMessages.options.type') || 'warning';
  }),

  elementId: 'flash',
  tagName: 'p',
  classNameBindings: ['message:shown', 'type'],
});
