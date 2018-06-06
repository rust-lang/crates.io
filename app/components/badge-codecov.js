import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: alias('badge.attributes.repository'),
    branch: computed('badge.attributes.branch', function() {
        return this.get('badge.attributes.branch') || 'master';
    }),
    service: computed('badge.attributes.service', function() {
        return this.get('badge.attributes.service') || 'github';
    }),
    text: computed('branch', function() {
        return `CodeCov coverage status for the ${this.branch} branch`;
    }),
});
