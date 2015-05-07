import Ember from 'ember';
const { computed } = Ember;

export default Ember.ObjectController.extend({
    currentPlatform: computed(function() {
        var os = null;

        if (navigator.platform === "Linux x86_64") {
            os = "x86_64-unknown-linux-gnu";
        } else if (navigator.platform === "Linux i686") {
            os = "i686-unknown-linux-gnu";
        }

        // I wish I knew by know, but I don't. Try harder.
        if (os == null) {
            if (navigator.appVersion.indexOf("Win") !== -1) {
                os = "i686-w64-mingw32";
            } else if (navigator.appVersion.indexOf("Mac") !== -1) {
                os = "x86_64-apple-darwin";
            } else if (navigator.appVersion.indexOf("Linux") !== -1) {
                os = "x86_64-unknown-linux-gnu";
            }
        }

        return os;
    }),

    downloadUrl: computed('currentPlatform', function() {
        var plat = this.get('currentPlatform');
        if (plat == null) { return null; }
        return `https://static.rust-lang.org/cargo-dist/cargo-nightly-${plat}.tar.gz`;
    })
});
