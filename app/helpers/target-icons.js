import Ember from 'ember';

export function targetIcons(targets) {
    return Ember.String.htmlSafe(targets.map(function(target) {
        const filename = target.split(' ')[0].toLowerCase();
        return `<img src="/assets/${filename}.svg" alt="${target}" />`;
    }).join(''));
}

export default Ember.Helper.helper(params => targetIcons(params[0]));
