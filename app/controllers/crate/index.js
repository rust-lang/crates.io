import Ember from 'ember';
import ajax from 'ic-ajax';

export default Ember.ObjectController.extend({
    isLoading: false,
    actions: {
        download: function(version) {
            this.set('isLoading', true);
            var self = this;
            var pkg_downloads = this.get('model').get('downloads');
            var ver_downloads = version.get('downloads');
            return ajax(version.get('dl_path')).then(function(data) {
                self.get('model').set('downloads', pkg_downloads + 1);
                version.set('downloads', ver_downloads + 1);
                Ember.$('#download-frame').attr('src', data.url);
            }).finally(function() {
                self.set('isLoading', false);
            });
        }
    }
});

