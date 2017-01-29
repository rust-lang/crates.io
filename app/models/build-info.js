import DS from 'ember-data';
import Ember from 'ember';

const TIER1 = {
    'x86_64-unknown-linux-gnu': 'Linux',
    'x86_64-apple-darwin': 'macOS',
    'x86_64-pc-windows-gnu': 'Windows (GNU)',
    'x86_64-pc-windows-msvc': 'Windows (MSVC)',
};

const caseInsensitive = (a, b) => a.toLowerCase().localeCompare(b.toLowerCase());

export default DS.Model.extend({
    version: DS.belongsTo('version', { async: true }),
    ordering: DS.attr(),
    stable: DS.attr(),
    beta: DS.attr(),
    nightly: DS.attr(),

    latest_positive_results: Ember.computed('ordering', 'stable', 'beta', 'nightly', function() {
        const passingTargets = results => (
            Object.entries(results)
                .filter(([, value]) => value === true)
                .map(([key]) => TIER1[key])
                .sort(caseInsensitive)
        );

        const positiveResults = (versionOrdering, channelResults) => {
            const latestVersion = versionOrdering[versionOrdering.length - 1];
            const latestResults = channelResults[latestVersion] || {};
            return [latestVersion, passingTargets(latestResults)];
        };

        let results = {};

        const addChannelToResults = (key) => {
            const channelOrdering = this.get('ordering')[key];
            const channelResults = this.get(key);

            const [version, targets] = positiveResults(channelOrdering, channelResults);

            if (targets.length > 0) {
                results[key] = { version, targets };
            }
        };

        addChannelToResults('stable');
        addChannelToResults('beta');
        addChannelToResults('nightly');

        return results;
    }),

    has_any_positive_results: Ember.computed('latest_positive_results', function() {
        const results = this.get('latest_positive_results');
        return Object.keys(results).length > 0;
    }),
});
