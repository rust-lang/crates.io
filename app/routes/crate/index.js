import Ember from 'ember';
import ajax from 'ic-ajax';
import Version from 'cargo/models/version';
import Crate from 'cargo/models/crate';

export default Ember.Route.extend({
    setupController(controller, data) {
        if (data instanceof Crate) {
            data = {crate: data, version: null};
        } else if (data instanceof Version) {
            data = {crate: data.get('crate'), version: data.get('num')};
        }

        this._super(controller, data.crate);

        controller.set('showAllVersions', false);
        controller.set('fetchingDownloads', true);
        controller.set('fetchingFollowing', true);

        data.crate.get('keywords').then((keywords) => controller.set('keywords', keywords));

        if (this.session.get('currentUser')) {
            var url = '/api/v1/crates/' + data.crate.get('name') + '/following';

            ajax(url)
                .then((d) => controller.set('following', d.following))
                .finally(() => controller.set('fetchingFollowing', false));
        }

        // Try to find the requested version in the versions we fetch
        var max = data.crate.get('max_version');
        data.crate.get('versions').then((array) => {
            var hit = array.any((version) => {
                return data.version === version.get('num') ||
                           (data.version == null && version.get('num') === max);
            });

            if (!hit) {
                var msg = "Version `" + data.version + "` does not exist";
                this.controllerFor('application').set('flashError', msg);
                data.version = null;
            }
            controller.set('requestedVersion', data.version);

            array.forEach((version) => {
                if (data.version === version.get('num') ||
                    (data.version == null && version.get('num') === max)) {
                    controller.set('currentVersion', version);
                }
            });

        }).then(() => {
            return controller.get('model.version_downloads');
        }).then((downloads) => {
            var meta = downloads.get('meta');
            controller.set('fetchingDownloads', false);
            controller.send('renderChart', downloads, meta.extra_downloads);
        });
    }
});
