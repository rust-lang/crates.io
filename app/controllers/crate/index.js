import Ember from 'ember';
import DS from 'ember-data';
import ajax from 'ic-ajax';

var NUM_VERSIONS = 5;

export default Ember.ObjectController.extend({
    needs: ['application'],
    isDownloading: false,

    fetchingDownloads: true,
    fetchingFollowing: true,
    following: false,
    showAllVersions: false,
    currentVersion: null,
    requestedVersion: null,
    keywords: [],

    sortedVersions: function() {
        return this.get("model.versions");
    }.property('model.versions.[]'),

    smallSortedVersions: function() {
        return this.get('sortedVersions').slice(0, NUM_VERSIONS);
    }.property('sortedVersions'),

    hasMoreVersions: function() {
        return this.get("sortedVersions.length") > NUM_VERSIONS;
    }.property('sortedVersions'),

    anyLinks: function() {
      return this.get('homepage') ||
             this.get('wiki') ||
             this.get('mailing_list') ||
             this.get('documentation') ||
             this.get('repository');
    }.property('homepage', 'wiki', 'mailing_list', 'documentation', 'repository'),

    versionsCount: function() {
      return this.get('versions.length');
    }.property('versions.@each'),

    displayedAuthors: function() {
        var self = this;
        if (!this.get('currentVersion')) {
            return [];
        }
        return DS.PromiseArray.create({
            promise: this.get('currentVersion.authors').then(function(authors) {
                var ret = [];
                authors.forEach(function(author) {
                    ret.push(author);
                });
                var others = self.store.metadataFor('user');
                for (var i = 0; i < others.names.length; i++) {
                    ret.push({name: others.names[i]});
                }
                return ret;
            }),
        });
    }.property('currentVersion.authors.@each'),

    anyKeywords: function() {
        return this.get('keywords.length') > 0;
    }.property('keywords'),

    currentDependencies: function() {
        var deps = this.get('currentVersion.dependencies');
        if (deps === null) { return []; }
        return DS.PromiseArray.create({
            promise: deps.then(function(deps) {
                var non_dev = deps.filter(function(dep) {
                    return dep.get('kind') !== 'dev';
                });
                var map = {};
                var ret = [];
                non_dev.forEach(function(dep) {
                    if (!(dep.get('crate_id') in map)) {
                        map[dep.get('crate_id')] = 1;
                        ret.push(dep);
                    }
                });
                return ret;
            }),
        });
    }.property('currentVersion.dependencies'),

    currentDevDependencies: function() {
        var deps = this.get('currentVersion.dependencies');
        if (deps === null) { return []; }
        return DS.PromiseArray.create({
            promise: deps.then(function(deps) {
                return deps.filter(function(dep) {
                    return dep.get('kind') === 'dev';
                });
            }),
        });
    }.property('currentVersion.dependencies'),

    actions: {
        download: function(version) {
            this.set('isDownloading', true);
            var self = this;
            var crate_downloads = this.get('model').get('downloads');
            var ver_downloads = version.get('downloads');
            return ajax({
                url: version.get('dl_path'),
                dataType: 'json',
            }).then(function(data) {
                self.get('model').set('downloads', crate_downloads + 1);
                version.set('downloads', ver_downloads + 1);
                Ember.$('#download-frame').attr('src', data.url);
            }).finally(function() {
                self.set('isDownloading', false);
            });
        },

        toggleVersions: function() {
            this.get('controllers.application')
                .resetDropdownOption(this, 'showAllVersions');
        },

        toggleFollow: function() {
            this.set('fetchingFollowing', true);
            this.set('following', !this.get('following'));
            var url = '/api/v1/crates/' + this.get('model.name') + '/follow';
            var method;
            if (this.get('following')) {
                method = 'put';
            } else {
                method = 'delete';
            }
            var self = this;
            ajax({ method: method, url: url }).finally(function() {
                self.set('fetchingFollowing', false);
            });
        },

        renderChart: function(downloads, extra) {
            var dates = {};
            var versions = [];
            for (var i = 0; i < 90; i++) {
                var now = moment().subtract(i, 'days');
                dates[now.format('MMM D')] = {date: now, cnt: {}};
            }
            downloads.forEach(function(d) {
                var version_id = d.get('version.id');
                var key = moment(d.get('date')).utc().format('MMM D');
                if (dates[key]) {
                    var prev = dates[key].cnt[version_id] || 0;
                    dates[key].cnt[version_id] = prev + d.get('downloads');
                }
            });
            extra.forEach(function(d) {
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
                    versions.push({id: tmp[i].get('id'), num: tmp[i].get('num')});
                }
            }
            if (extra.length > 0) {
                versions.push({ id: null, num: 'Other' });
            }

            var headers = ['Date'];
            versions.sort(function(b) { return b.num; }).reverse();
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

            var drawChart = function() {
                if (!window.google || !window.googleChartsLoaded) {
                    Ember.$('.graph').hide();
                    return;
                } else {
                    Ember.$('.graph').show();
                }
                var myData = google.visualization.arrayToDataTable(data);

                var fmt = new google.visualization.DateFormat({
                    pattern: 'LLL d, yyyy',
                });
                fmt.format(myData, 0);
                var el = document.getElementById('graph-data');
                if (!el) {
                    return;
                }
                var chart = new google.visualization.AreaChart(el);
                chart.draw(myData, {
                    chartArea: {'width': '78%', 'height': '80%'},
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
            };

            Ember.run.scheduleOnce('afterRender', this, drawChart);
            Ember.$(window).off('resize.chart');
            Ember.$(window).on('resize.chart', drawChart);
            Ember.$(document).off('googleChartsLoaded');
            Ember.$(document).on('googleChartsLoaded', drawChart);
        },
    },
});

