import Ember from 'ember';

export default Ember.Component.extend({
    flashMessages: Ember.inject.service(),
    message: Ember.computed.readOnly('flashMessages.message'),

    elementId: 'flash',
    tagName: 'p',
    classNameBindings: ['message:shown']
});
