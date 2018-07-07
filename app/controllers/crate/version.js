import { alias, readOnly, gt } from '@ember/object/computed';
import { inject as service } from '@ember/service';
import Controller from '@ember/controller';
import PromiseProxyMixin from '@ember/object/promise-proxy-mixin';
import ArrayProxy from '@ember/array/proxy';
import { computed, observer } from '@ember/object';
import { later } from '@ember/runloop';
import $ from 'jquery';
import moment from 'moment';

const NUM_VERSIONS = 5;

const PromiseArray = ArrayProxy.extend(PromiseProxyMixin);

export default Controller.extend({
    session: service(),

    isDownloading: false,

    downloadsContext: computed('requestedVersion', 'model', 'crate', function() {
        return this.requestedVersion ? this.model : this.crate;
    }),
    downloads: alias('downloadsContext.version_downloads'),
    extraDownloads: alias('downloads.content.meta.extra_downloads'),

    fetchingFollowing: true,
    following: false,
    currentVersion: alias('model'),
    requestedVersion: null,
    keywords: alias('crate.keywords'),
    categories: alias('crate.categories'),
    badges: alias('crate.badges'),
    isOwner: computed('crate.owner_user', 'session.currentUser.login', function() {
        return this.get('crate.owner_user').findBy('login', this.get('session.currentUser.login'));
    }),

    sortedVersions: readOnly('crate.versions'),

    smallSortedVersions: computed('sortedVersions', function() {
        return this.sortedVersions.slice(0, NUM_VERSIONS);
    }),

    hasMoreVersions: gt('sortedVersions.length', NUM_VERSIONS),

    displayedAuthors: computed('currentVersion.authors.[]', function() {
        return PromiseArray.create({
            promise: this.get('currentVersion.authors').then(authors => {
                let ret = authors.slice();
                let others = authors.get('meta');
                for (let i = 0; i < others.names.length; i++) {
                    ret.push({ name: others.names[i] });
                }
                return ret;
            }),
        });
    }),

    anyKeywords: gt('keywords.length', 0),
    anyCategories: gt('categories.length', 0),

    currentDependencies: computed('currentVersion.dependencies', function() {
        let deps = this.get('currentVersion.dependencies');

        if (deps === null) {
            return [];
        }

        return PromiseArray.create({
            promise: deps.then(deps => {
                return deps.filter(dep => dep.get('kind') !== 'dev').uniqBy('crate_id');
            }),
        });
    }),

    currentDevDependencies: computed('currentVersion.dependencies', function() {
        let deps = this.get('currentVersion.dependencies');
        if (deps === null) {
            return [];
        }
        return PromiseArray.create({
            promise: deps.then(deps => {
                return deps.filterBy('kind', 'dev');
            }),
        });
    }),

    downloadData: computed('downloads', 'extraDownloads', 'requestedVersion', function() {
        let downloads = this.downloads;
        if (!downloads) {
            return;
        }

        let extra = this.extraDownloads || [];

        let dates = {};
        let versions = [];
        for (let i = 0; i < 90; i++) {
            let now = moment().subtract(i, 'days');
            dates[now.format('MMM D')] = { date: now, cnt: {} };
        }

        downloads.forEach(d => {
            let version_id = d.get('version.id');
            let key = moment(d.get('date'))
                .utc()
                .format('MMM D');
            if (dates[key]) {
                let prev = dates[key].cnt[version_id] || 0;
                dates[key].cnt[version_id] = prev + d.get('downloads');
            }
        });

        extra.forEach(d => {
            let key = moment(d.date)
                .utc()
                .format('MMM D');
            if (dates[key]) {
                let prev = dates[key].cnt[null] || 0;
                dates[key].cnt[null] = prev + d.downloads;
            }
        });
        if (this.requestedVersion) {
            versions.push(this.model.getProperties('id', 'num'));
        } else {
            this.smallSortedVersions.forEach(version => {
                versions.push(version.getProperties('id', 'num'));
            });
        }
        if (extra.length > 0) {
            versions.push({
                id: null,
                num: 'Other',
            });
        }

        let headers = ['Date'];
        versions.sort(b => b.num).reverse();
        for (let i = 0; i < versions.length; i++) {
            headers.push(versions[i].num);
        }
        let data = [headers];
        for (let date in dates) {
            let row = [dates[date].date.toDate()];
            for (let i = 0; i < versions.length; i++) {
                row.push(dates[date].cnt[versions[i].id] || 0);
            }
            data.push(row);
        }

        return data;
    }),

    toggleClipboardProps(isSuccess) {
        this.setProperties({
            showSuccess: isSuccess,
            showNotification: true,
        });
        later(
            this,
            () => {
                this.set('showNotification', false);
            },
            2000,
        );
    },

    actions: {
        copySuccess(event) {
            event.clearSelection();
            this.toggleClipboardProps(true);
        },

        copyError() {
            this.toggleClipboardProps(false);
        },

        toggleFollow() {
            this.set('fetchingFollowing', true);

            let crate = this.crate;
            let op = this.toggleProperty('following') ? crate.follow() : crate.unfollow();

            return op.finally(() => this.set('fetchingFollowing', false));
        },

        showDeps() {
            var margin = {top: window.innerHeight / 100 * 3,
                          right: (window.innerWidth / 100 * 6),
                          bottom: window.innerHeight / 100 * 3,
                          left: (window.innerWidth / 100 * 6)},
                width = 1950,
                height = window.innerHeight - (window.innerHeight / 100 * 6);

            var i = 0,
                duration = 750,
                root = {
                    crate_id: this.get('crate.name'),
                    version_id: this.get('currentVersion.num'),
                    level: 1,
                };

            var tree = d3.layout.tree().size([height, width]);

            var diagonal = d3.svg.diagonal()
                                 .projection(function(d) {
                                     return [d.y, d.x];
                                 });

            var svg = d3.select("#deps-rendering")
                        .attr("width", width)
                        .attr("height", height)
                        .append("g")
                        .attr("transform", "translate(" + margin.left + "," + margin.top + ")");

            // Load dependencies.
            //root = flare;
            root.x0 = height / 2;
            root.y0 = 0;

            var total = 0;
            var crates = {};

            function get_wanted_version_num(object, wanted_version) {
                for (var i = 0; i < object.versions.length; ++i) {
                    if (object.versions[i].id == wanted_version) {
                        return object.versions[i].num;
                    }
                }
                return object.versions[0].num;
            }

            function decrement_total(level) {
                total -= 1;
                /*if (Object.keys(crates).length % 20 === 0) {
                    console.log(crates);
                }*/
                if (total === 0 || level < 3) {
                    update(root);
                    if (total === 0) {
                        console.log(root);
                    }
                }
            }

            function add_dependencies(e, children) {
                for (var i = 0; i < children.length; ++i) {
                    children[i].level = e.level + 1;
                }
                e.children = null;
                e._children = children;
                if (e._children && e.level < 4) {
                    e._children.forEach(collapse);
                }
                if (e.level < 3) {
                    e.children = e._children;
                    e._children = null;
                }
                decrement_total(e.level);
            }

            var all_urls = {};

            function get_next_url(url, callback) {
                if (all_urls[url] === undefined) {
                    all_urls[url] = {'callbacks': [callback], 'pending': true};
                    d3.json(url, (error, flare) => {
                        if (error) {
                            decrement_total();
                            all_urls[url] = undefined;
                            return;
                        }
                        var callbacks = all_urls[url].callbacks;
                        all_urls[url] = flare;
                        all_urls[url].pending = false;
                        for (var i = 0; i < callbacks.length; ++i) {
                            callbacks[i](flare);
                        }
                    });
                } else if (all_urls[url].pending === true) {
                    all_urls[url].callbacks.push(callback);
                } else {
                    callback(all_urls[url]);
                }
            }

            function get_dependencies_for_version(crate, version, e) {
                var tmp = e;
                get_next_url(`/api/v1/crates/${crate}/${version}/dependencies`, (data) => {
                    tmp.version_id = version;
                    add_dependencies(tmp, JSON.parse(JSON.stringify(data.dependencies)));
                });
            }

            function collapse(d) {
                if (!d.children && d.optional !== true) {
                    total += 1;
                    if (d.version_id.toString().indexOf(".") !== -1) {
                        get_dependencies_for_version(d.crate_id, d.version_id, d);
                    } else {
                        get_next_url(`/api/v1/crates/${d.crate_id}`, (data) => {
                            var ver = get_wanted_version_num(data, d.version_id);
                            get_dependencies_for_version(d.crate_id, ver, d);
                        });
                    }
                }
            }

            collapse(root);

            //d3.select(self.frameElement).style("height", "800px");

            function update(source) {
                // Compute the new tree layout.
                var nodes = tree.nodes(source).reverse(),
                    links = tree.links(nodes);

                // Normalize for fixed-depth.
                nodes.forEach(function(d) {
                    d.y = d.depth * 180;
                });

                // Update the nodes...
                var node = svg.selectAll("g.node")
                              .data(nodes, function(d) {
                                  return d.id || (d.id = ++i);
                              });

                // Enter any new nodes at the parent's previous position.
                var nodeEnter = node.enter()
                                    .append("g")
                                    .attr("class", "node")
                                    .attr("transform", function(d) {
                                        return "translate(" + source.y0 + "," + source.x0 + ")";
                                    })
                                    .on("click", click);

                nodeEnter.append("circle")
                         .attr("r", 1e-6)
                         .style("fill", function(d) {
                             return d._children ? "lightsteelblue" : "#fff";
                         });

                nodeEnter.append("text")
                         .attr("x", function(d) {
                             return d.children || d._children ? -10 : 10;
                         })
                         .attr("dy", ".35em")
                         .attr("text-anchor", function(d) {
                             return d.children || d._children ? "end" : "start";
                         })
                         .text(function(d) {
                             return d.crate_id + ' ' + d.version_id;
                         })
                         .style("fill-opacity", 1e-6);

                // Transition nodes to their new position.
                var nodeUpdate = node.transition()
                                     .duration(duration)
                                     .attr("transform", function(d) {
                                         return "translate(" + d.y + "," + d.x + ")";
                                     });

                nodeUpdate.select("circle")
                          .attr("r", 4.5)
                          .style("fill", function(d) {
                              return d._children ? "lightsteelblue" : "#fff";
                          });

                nodeUpdate.select("text")
                          .style("fill-opacity", 1);

                // Transition exiting nodes to the parent's new position.
                var nodeExit = node.exit()
                                   .transition()
                                   .duration(duration)
                                   .attr("transform", function(d) {
                                       return "translate(" + source.y + "," + source.x + ")";
                                   })
                                   .remove();

                nodeExit.select("circle")
                        .attr("r", 1e-6);

                nodeExit.select("text")
                        .style("fill-opacity", 1e-6);

                // Update the links…
                var link = svg.selectAll("path.link")
                              .data(links, function(d) { return d.target.id; });

                // Enter any new links at the parent's previous position.
                link.enter()
                    .insert("path", "g")
                    .attr("class", "link")
                    .attr("d", function(d) {
                        var o = {x: source.x0, y: source.y0};
                        return diagonal({source: o, target: o});
                    });

                // Transition links to their new position.
                link.transition()
                    .duration(duration)
                    .attr("d", diagonal);

                // Transition exiting nodes to the parent's new position.
                link.exit()
                    .transition()
                    .duration(duration)
                    .attr("d", function(d) {
                        var o = {x: source.x, y: source.y};
                        return diagonal({source: o, target: o});
                    })
                    .remove();

                // Stash the old positions for transition.
                nodes.forEach(function(d) {
                    d.x0 = d.x;
                    d.y0 = d.y;
                });
            }

            // Toggle children on click.
            function click(d) {
                if (d.children) {
                    d._children = d.children;
                    d.children = null;
                } else {
                    d.children = d._children;
                    d._children = null;
                }
                update(root);
            }
            this.set('showingDeps', true);
        }
    },

    report: observer('crate.readme', function() {
        setTimeout(() => $(window).trigger('hashchange'));
    }),
});
