import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
    user: null,
    attributeBindings: ['title', 'href'],
    tagName: 'a',

    title: computed.readOnly('user.login'),

    // TODO replace this with a link to a native crates.io profile
    // page when they exist.
    href: computed.readOnly('user.url'),
});
