import Ember from 'ember';

export default Ember.Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: Ember.computed.alias('badge.attributes.repository'),
    branch: Ember.computed('badge.attributes.branch', function() {
        return this.get('badge.attributes.branch') || 'master';
    }),
    service: Ember.computed('badge.attributes.service', function() {
        return this.get('badge.attributes.service') || 'github';
    }),
    text: Ember.computed('branch', function() {
        return `CodeCov coverage status for the ${ this.get('branch') } branch`;
    })
});
