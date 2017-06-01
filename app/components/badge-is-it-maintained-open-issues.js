import Ember from 'ember';

export default Ember.Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: Ember.computed.alias('badge.attributes.repository'),
    text: Ember.computed('badge', function() {
        return `Is It Maintained percentage of issues still open`;
    })
});
