import { readOnly } from '@ember/object/computed';
import Component from '@ember/component';

export default Component.extend({
    user: null,
    attributeBindings: ['title', 'href'],
    tagName: 'a',

    title: readOnly('user.login'),

    // TODO replace this with a link to a native crates.io profile
    // page when they exist.
    href: readOnly('user.url'),
});
