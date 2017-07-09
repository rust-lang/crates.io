import Ember from 'ember';

const { computed, inject: { service } } = Ember;

export default Ember.Mixin.create({

    fastboot: service(),

    isNotFastBoot: computed.not('fastboot.isFastBoot'),

    /**
     * When there's a need to raise network requests from the server
     * during Server Side Rendering(SSR) via FastBoot,
     * the JS Fetch API doesn't work with relative urls.
     * This property gives the URL prefix for those network calls.
     * */
    appURL: computed(function() {
        let url = '';
        if (this.get('fastboot.isFastBoot')) {
            let protocol = this.get('fastboot.request.protocol');
            let host = this.get('fastboot.request.host');
            url = `${protocol}://${host}`;
        }
        return url;
    }),
});
