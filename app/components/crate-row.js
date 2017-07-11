import Component from '@ember/component';
import { computed } from '@ember/object';

export default Component.extend({
    classNames: ['crate', 'row'],
    sort: null,

    isDownloads: computed('sort', function() {
        if (this.get('sort') === 'All-Time Downloads') {
            return true;
        } else {
            return false;
        }
    }),

    isRecentDownloads: computed('sort', function() {
        if (this.get('sort') === 'Recent Downloads') {
            return true;
        } else {
            return false;
        }
    }),

    isAlpha: computed('sort', function() {
        if (this.get('sort') === 'Alphabetical') {
            return true;
        } else {
            return false;
        }
    })
});
