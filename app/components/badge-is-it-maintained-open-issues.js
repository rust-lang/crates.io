import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
    tagName: 'span',
    classNames: ['badge'],
    repository: alias('badge.attributes.repository'),
    text: computed('badge', function() {
        return `Is It Maintained percentage of issues still open`;
    }),
});
