import Ember from 'ember';
import DS from 'ember-data';
import ajax from 'ic-ajax';

var NUM_VERSIONS = 5;

export default Ember.ObjectController.extend({
    needs: ['application'],
    isDownloading: false,

    fetchingVersions: true,
    fetchingDownloads: true,
    fetchingFollowing: true,
    following: false,
    showAllVersions: false,
    currentVersion: null,
    requestedVersion: null,

    sortedVersions: function() {
        return this.get("model").get("versions").sortBy("num").reverse();
    }.property('model.versions.[]'),

    smallSortedVersions: function() {
        return this.get('sortedVersions').slice(0, NUM_VERSIONS);
    }.property('sortedVersions'),

    hasMoreVersions: function() {
        return this.get("sortedVersions").length > NUM_VERSIONS;
    }.property('sortedVersions'),

    displayedAuthors: function() {
        var self = this;
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
            var url;
            if (this.get('following')) {
                url = '/crates/' + this.get('model').get('name') + '/follow';
            } else {
                url = '/crates/' + this.get('model').get('name') + '/unfollow';
            }
            var self = this;
            ajax({ method: 'put', url: url }).finally(function() {
                self.set('fetchingFollowing', false);
            });
        },

        renderChart: function(downloads) {
            var dates = {};
            for (var i = 0; i < 90; i++) {
                var now = moment().subtract(i, 'days');
                dates[now.format('MMM D')] = {date: now, cnt: 0};
            }
            downloads.forEach(function(d) {
                var key = moment(d.get('date')).utc().format('MMM D');
                if (dates[key]) {
                    dates[key].cnt += d.get('downloads');
                }
            });

            var data = [['Date', this.get('currentVersion').get('num')]];
            for (var date in dates) {
                data.push([dates[date].date.toDate(), dates[date].cnt]);
            }
            data = google.visualization.arrayToDataTable(data);

            Ember.run.scheduleOnce('afterRender', this, function() {
                var el = document.getElementById('graph-data');
                var chart = new google.visualization.LineChart(el);
                chart.draw(data, {
                    chartArea: {'width': '80%', 'height': '80%'},
                    hAxis: {
                        minorGridlines: { count: 8 },
                    },
                    vAxis: {
                        minorGridlines: { count: 5 },
                        viewWindow: { min: 0, },
                    },
                });
            });
        },
    },
});

