import DS from 'ember-data';
import Ember from 'ember';

const TIER1 = [
    ['x86_64-unknown-linux-gnu', 'Linux'],
    ['x86_64-apple-darwin', 'macOS'],
    ['x86_64-pc-windows-gnu', 'Windows (GNU)'],
    ['x86_64-pc-windows-msvc', 'Windows (MSVC)'],
];

const last = name => (
    Ember.computed(name, function() {
        const items = this.get(name);
        return items[items.length - 1];
    })
);

export default DS.Model.extend({
    version: DS.belongsTo('version', { async: true }),
    ordering: DS.attr(),
    stable: DS.attr(),
    beta: DS.attr(),
    nightly: DS.attr(),

    has_any_info: Ember.computed('ordering', function() {
        const ordering = this.get('ordering');
        const num_results = ordering.stable.length + ordering.nightly.length + ordering.beta.length;
        return num_results > 0;
    }),

    latest_stable: last('ordering.stable'),
    latest_beta: last('ordering.beta'),
    latest_nightly: last('ordering.nightly'),
    tier1_results: Ember.computed('nightly', 'latest_nightly', 'beta', 'latest_beta', 'stable', 'latest_stable', function() {
        const nightly_results = this.get('nightly')[this.get('latest_nightly')] || {};
        const beta_results = this.get('beta')[this.get('latest_beta')] || {};
        const stable_results = this.get('stable')[this.get('latest_stable')] || {};

        return TIER1.map(([target, display]) => ({
            display_target: display,
            nightly: nightly_results[target],
            beta: beta_results[target],
            stable: stable_results[target]
        }));
    }),
});
