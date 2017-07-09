import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
    size: 'small',
    user: null,
    attributeBindings: ['src', 'width', 'height'],
    tagName: 'img',

    width: computed('size', function() {
        if (this.get('size') === 'small') {
            return 22;
        } else if (this.get('size') === 'medium-small') {
            return 32;
        } else {
            return 85; // medium
        }
    }),

    height: computed.readOnly('width'),

    src: computed('size', 'user', function() {
        return `${this.get('user.avatar')}&s=${this.get('width') * 2}`;
    })
});
