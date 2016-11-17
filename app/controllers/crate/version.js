import Ember from 'ember';
import DS from 'ember-data';
import moment from 'moment';

const NUM_VERSIONS = 5;
const { computed } = Ember;

export default Ember.Controller.extend({
    isDownloading: false,

    downloadsContext: computed('requestedVersion', 'model', 'crate', function() {
        return this.get('requestedVersion') ? this.get('model') : this.get('crate');
    }),
    downloads: computed.alias('downloadsContext.version_downloads'),
    extraDownloads: computed.alias('downloads.content.meta.extra_downloads'),

    fetchingFollowing: true,
    following: false,
    currentVersion: computed.alias('model'),
    requestedVersion: null,
    keywords: computed.alias('crate.keywords'),
    categories: computed.alias('crate.categories'),

    sortedVersions: computed.readOnly('crate.versions'),

    smallSortedVersions: computed('sortedVersions', function() {
        return this.get('sortedVersions').slice(0, NUM_VERSIONS);
    }),

    hasMoreVersions: computed.gt('sortedVersions.length', NUM_VERSIONS),

    anyLinks: computed.or('crate.homepage',
                          'crate.wiki',
                          'crate.mailing_list',
                          'crate.documentation',
                          'crate.repository',
                          'crate.reverse_dependencies'),

    displayedAuthors: computed('currentVersion.authors.[]', function() {
        return DS.PromiseArray.create({
            promise: this.get('currentVersion.authors').then((authors) => {
                var ret = authors.slice();
                var others = authors.get('meta');
                for (var i = 0; i < others.names.length; i++) {
                    ret.push({ name: others.names[i] });
                }
                return ret;
            })
        });
    }),

    anyKeywords: computed.gt('keywords.length', 0),
    anyCategories: computed.gt('categories.length', 0),

    currentDependencies: computed('currentVersion.dependencies', function() {
        var deps = this.get('currentVersion.dependencies');

        if (deps === null) {
            return [];
        }

        return DS.PromiseArray.create({
            promise: deps.then((deps) => {
                var non_dev = deps.filter((dep) => dep.get('kind') !== 'dev');
                var map = {};
                var ret = [];

                non_dev.forEach((dep) => {
                    if (!(dep.get('crate_id') in map)) {
                        map[dep.get('crate_id')] = 1;
                        ret.push(dep);
                    }
                });

                return ret;
            })
        });
    }),

    currentDevDependencies: computed('currentVersion.dependencies', function() {
        var deps = this.get('currentVersion.dependencies');
        if (deps === null) {
            return [];
        }
        return DS.PromiseArray.create({
            promise: deps.then((deps) => {
                return deps.filterBy('kind', 'dev');
            }),
        });
    }),

    actions: {
        download(version) {
            this.set('isDownloading', true);

            version.getDownloadUrl().then(url => {
                this.incrementProperty('crate.downloads');
                this.incrementProperty('currentVersion.downloads');
                Ember.$('#download-frame').attr('src', url);
            }).finally(() => this.set('isDownloading', false));
        },

        toggleFollow() {
            this.set('fetchingFollowing', true);

            let crate = this.get('crate');
            let op = this.toggleProperty('following') ?
                crate.follow() : crate.unfollow();

            return op.finally(() => this.set('fetchingFollowing', false));
        },
    },

    downloadData: computed('downloads', 'extraDownloads', 'requestedVersion', function() {
        let downloads = this.get('downloads');
        if (!downloads) {
            return;
        }

        let extra = this.get('extraDownloads') || [];

        var dates = {};
        var versions = [];
        for (var i = 0; i < 90; i++) {
            var now = moment().subtract(i, 'days');
            dates[now.format('MMM D')] = { date: now, cnt: {} };
        }

        downloads.forEach((d) => {
            var version_id = d.get('version.id');
            var key = moment(d.get('date')).utc().format('MMM D');
            if (dates[key]) {
                var prev = dates[key].cnt[version_id] || 0;
                dates[key].cnt[version_id] = prev + d.get('downloads');
            }
        });

        extra.forEach((d) => {
            var key = moment(d.date).utc().format('MMM D');
            if (dates[key]) {
                var prev = dates[key].cnt[null] || 0;
                dates[key].cnt[null] = prev + d.downloads;
            }
        });
        if (this.get('requestedVersion')) {
            versions.push(this.get('model').getProperties('id', 'num'));
        } else {
            this.get('smallSortedVersions').forEach(version => {
                versions.push(version.getProperties('id', 'num'));
            });
        }
        if (extra.length > 0) {
            versions.push({
                id: null,
                num: 'Other'
            });
        }

        var headers = ['Date'];
        versions.sort((b) => b.num).reverse();
        for (i = 0; i < versions.length; i++) {
            headers.push(versions[i].num);
        }
        var data = [headers];
        for (var date in dates) {
            var row = [dates[date].date.toDate()];
            for (i = 0; i < versions.length; i++) {
                row.push(dates[date].cnt[versions[i].id] || 0);
            }
            data.push(row);
        }

        return data;
    }),
});
