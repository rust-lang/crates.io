import Ember from 'ember';

export default Ember.Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: Ember.computed.alias('badge.attributes.repository'),
    branch: Ember.computed('badge.attributes.branch', function() {
        return encodeURIComponent(this.get('badge.attributes.branch') || 'master');
    }),
    text: Ember.computed('branch', function() {
        return `Circle CI build status for the ${ this.get('branch') } branch`;
    })
});
