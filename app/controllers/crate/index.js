import Ember from 'ember';
import DS from 'ember-data';
import ajax from 'ic-ajax';
import moment from 'moment';

const NUM_VERSIONS = 5;
const { computed } = Ember;

export default Ember.Controller.extend({
    applicationController: Ember.inject.controller('application'),
    isDownloading: false,

    fetchingDownloads: true,
    fetchingFollowing: true,
    following: false,
    showAllVersions: false,
    currentVersion: null,
    requestedVersion: null,
    keywords: [],

    sortedVersions: computed('model.versions.[]', function() {
        return this.get("model.versions");
    }),

    smallSortedVersions: computed('sortedVersions', function() {
        return this.get('sortedVersions').slice(0, NUM_VERSIONS);
    }),

    hasMoreVersions: computed.gt('sortedVersions.length', NUM_VERSIONS),

    anyLinks: computed.or('model.homepage',
                          'model.wiki',
                          'model.mailing_list',
                          'model.documentation',
                          'model.repository'),

    displayedAuthors: computed('currentVersion.authors.[]', function() {
        if (!this.get('currentVersion')) {
            return [];
        }

        return DS.PromiseArray.create({
            promise: this.get('currentVersion.authors').then((authors) => {
                var ret = authors.slice();
                var others = this.store.metadataFor('user');
                for (var i = 0; i < others.names.length; i++) {
                    ret.push({name: others.names[i]});
                }
                return ret;
            })
        });
    }),

    anyKeywords: computed.gt('keywords.length', 0),

    currentDependencies: computed('currentVersion.dependencies', function() {
        var deps = this.get('currentVersion.dependencies');

        if (deps === null) { return []; }

        return DS.PromiseArray.create({
            promise: deps.then((deps) => {
                var non_dev = deps.filter((dep) => dep.get('kind') !== 'dev' );
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
        if (deps === null) { return []; }
        return DS.PromiseArray.create({
            promise: deps.then((deps) => {
                return deps.filterBy('kind', 'dev');
            }),
        });
    }),

    actions: {
        download(version) {
            this.set('isDownloading', true);

            var crate_downloads = this.get('model').get('downloads');
            var ver_downloads = version.get('downloads');

            return ajax({
                url: version.get('dl_path'),
                dataType: 'json',
            }).then((data) => {
                this.get('model').set('downloads', crate_downloads + 1);
                version.set('downloads', ver_downloads + 1);
                Ember.$('#download-frame').attr('src', data.url);
            }).finally(() => this.set('isDownloading', false) );
        },

        toggleVersions() {
            this.get('applicationController')
                .resetDropdownOption(this, 'showAllVersions');
        },

        toggleFollow() {
            this.set('fetchingFollowing', true);
            this.set('following', !this.get('following'));
            var url = '/api/v1/crates/' + this.get('model.name') + '/follow';
            var method;
            if (this.get('following')) {
                method = 'put';
            } else {
                method = 'delete';
            }

            ajax({
                method,
                url
            }).finally(() => this.set('fetchingFollowing', false));
        },

        renderChart(downloads, extra) {
            var dates = {};
            var versions = [];
            for (var i = 0; i < 90; i++) {
                var now = moment().subtract(i, 'days');
                dates[now.format('MMM D')] = {date: now, cnt: {}};
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
                versions.push({
                    id: this.get('currentVersion.id'),
                    num: this.get('currentVersion.num'),
                });
            } else {
                var tmp = this.get('smallSortedVersions');
                for (i = 0; i < tmp.length; i++) {
                    versions.push({
                      id: tmp[i].get('id'),
                      num: tmp[i].get('num')
                    });
                }
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

            // TODO: move this to a component
            function drawChart() {
                if (!window.google || !window.googleChartsLoaded) {
                    Ember.$('.graph').hide();
                    return;
                } else {
                    Ember.$('.graph').show();
                }
                var myData = window.google.visualization.arrayToDataTable(data);

                var fmt = new window.google.visualization.DateFormat({
                    pattern: 'LLL d, yyyy',
                });
                fmt.format(myData, 0);
                var el = document.getElementById('graph-data');
                if (!el) {
                    return;
                }
                var chart = new window.google.visualization.AreaChart(el);
                chart.draw(myData, {
                    chartArea: {'left': 85, 'width': '77%', 'height': '80%'},
                    hAxis: {
                        minorGridlines: { count: 8 },
                    },
                    vAxis: {
                        minorGridlines: { count: 5 },
                        viewWindow: { min: 0, },
                    },
                    isStacked: true,
                    focusTarget: 'category',
                });
            }

            Ember.run.scheduleOnce('afterRender', this, drawChart);
            Ember.$(window).off('resize.chart');
            Ember.$(window).on('resize.chart', drawChart);
            Ember.$(document).off('googleChartsLoaded');
            Ember.$(document).on('googleChartsLoaded', drawChart);
        },
    },
});
