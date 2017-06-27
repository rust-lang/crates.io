import Ember from 'ember';

const { computed, inject: { service } } = Ember;

export default Ember.Mixin.create({

    fastboot: service(),

    isNotFastBoot: computed.not('fastboot.isFastBoot'),

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
