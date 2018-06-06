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
    text: computed('badge', function() {
        return `GitLab build status for the ${this.branch} branch`;
    }),
});
