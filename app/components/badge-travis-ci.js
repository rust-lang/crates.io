import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: alias('badge.attributes.repository'),
    tld: computed('badge.attributes.tld', function() {
        let tld = this.get('badge.attributes.tld');
        switch (tld) {
            case 'org':
                return 'org';
            case 'com':
                return 'com';
            default:
                return 'org';
        }
    }),
    branch: computed('badge.attributes.branch', function() {
        return this.get('badge.attributes.branch') || 'master';
    }),
    text: computed('branch', function() {
        return `Travis CI build status for the ${this.branch} branch`;
    }),
});
