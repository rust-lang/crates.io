import Component from '@ember/component';
import { readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

export default Component.extend({
    flashMessages: service(),
    message: readOnly('flashMessages.message'),

    elementId: 'flash',
    tagName: 'p',
    classNameBindings: ['message:shown'],
});
