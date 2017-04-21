import Ember from 'ember';

const { computed } = Ember;

function flattenBuildInfo(buildOrdering, builds) {
    if (!buildOrdering || !builds) {
        return [];
    }

    return buildOrdering.map(version => {
        const thisVersion = builds[version];

        return {
            version,
            'x86_64-apple-darwin': thisVersion['x86_64-apple-darwin'],
            'x86_64-pc-windows-gnu': thisVersion['x86_64-pc-windows-gnu'],
            'x86_64-pc-windows-msvc': thisVersion['x86_64-pc-windows-msvc'],
            'x86_64-unknown-linux-gnu': thisVersion['x86_64-unknown-linux-gnu'],
        };
    });
}

export default Ember.Controller.extend({
    id: computed.alias('model.crate.id'),
    name: computed.alias('model.crate.name'),
    version: computed.alias('model.num'),
    build_info: computed.alias('model.build_info'),
    stable_build: computed('build_info.ordering.stable', 'build_info.stable', function() {
        const ordering = this.get('build_info.ordering.stable');
        const stable = this.get('build_info.stable');

        return flattenBuildInfo(ordering, stable);
    }),
    beta_build: computed('build_info.ordering.beta', 'build_info.beta', function() {
        const ordering = this.get('build_info.ordering.beta');
        const beta = this.get('build_info.beta');

        return flattenBuildInfo(ordering, beta);
    }),
    nightly_build: computed('build_info.ordering.nightly', 'build_info.nightly', function() {
        const ordering = this.get('build_info.ordering.nightly');
        const nightly = this.get('build_info.nightly');

        return flattenBuildInfo(ordering, nightly);
    }),
    has_stable_builds: computed.gt('stable_build.length', 0),
    has_beta_builds: computed.gt('beta_build.length', 0),
    has_nightly_builds: computed.gt('nightly_build.length', 0),
});
