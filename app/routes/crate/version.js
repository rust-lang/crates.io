import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.Route.extend({

    model(params) {
        const requestedVersion = params.version_num;

        const crate = this.modelFor('crate');
        const controller = this.controllerFor('crate.version');
        const maxVersion = crate.get('max_version');

        // Fall back to the crate's `max_version` property
        if (!requestedVersion) {
            params.version_num = maxVersion;
        }

        controller.set('crate', crate);
        controller.set('requestedVersion', requestedVersion);
        controller.set('fetchingDownloads', true);
        controller.set('fetchingFollowing', true);

        crate.get('keywords')
            .then((keywords) => controller.set('keywords', keywords));

        if (this.session.get('currentUser')) {
            ajax(`/api/v1/crates/${crate.get('name')}/following`)
                .then((d) => controller.set('following', d.following))
                .finally(() => controller.set('fetchingFollowing', false));
        }

        // Find version model
        return crate.get('versions')
            .then(versions => {
                const version = versions.find(version => version.get('num') === params.version_num);
                if (!version) {
                    this.controllerFor('application').set('nextFlashError',
                        `Version '${params.version_num}' of crate '${crate.get('name')}' does not exist`);
                }

                return version ||
                    versions.find(version => version.get('num') === maxVersion) ||
                    versions.objectAt(0);
            });
    },

    // can't do this in setupController because it won't be called
    // when going from "All Versions" to the current version
    afterModel(model) {
        this._super(...arguments);

        const controller = this.controllerFor('crate.version');
        const context = controller.get('requestedVersion') ? model : this.modelFor('crate');

        context.get('version_downloads').then(downloads => {
            controller.set('fetchingDownloads', false);
            controller.set('downloads', downloads);
            controller.set('extraDownloads', downloads.get('meta.extra_downloads') || []);
        });
    },

    serialize(model) {
        if (!model) {
            return { version_num: '' };
        } else {
            return { version_num: model.get('num') };
        }
    },
});
