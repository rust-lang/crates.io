import Ember from 'ember';

import { formatDay } from 'cargo/helpers/format-day';

export default Ember.Component.extend({
    tagName: 'span',
    classNames: ['build_info'],

    build_info: Ember.computed('crate.max_build_info_stable', 'crate.max_build_info_beta', 'crate.max_build_info_nightly', function() {
        if (this.get('crate.max_build_info_stable')) {
            return 'stable';
        } else if (this.get('crate.max_build_info_beta')) {
            return 'beta';
        } else if (this.get('crate.max_build_info_nightly')) {
            return 'nightly';
        } else {
            return null;
        }
    }),
    color: Ember.computed('build_info', function() {
        if (this.get('build_info') === 'stable') {
            return 'brightgreen';
        } else if (this.get('build_info') === 'beta') {
            return 'yellow';
        } else {
            return 'orange';
        }
    }),
    version_display: Ember.computed('build_info', 'crate.max_build_info_stable', 'crate.max_build_info_beta', 'crate.max_build_info_nightly', function() {
        if (this.get('build_info') === 'stable') {
            return this.get('crate.max_build_info_stable');
        } else if (this.get('build_info') === 'beta') {
            return formatDay(this.get('crate.max_build_info_beta'));
        } else {
            return formatDay(this.get('crate.max_build_info_nightly'));
        }
    }),
    version_for_shields: Ember.computed('version_display', function() {
        return this.get('version_display').replace(/-/g, '--');
    }),
});
