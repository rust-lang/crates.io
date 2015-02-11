import Ember from 'ember';

export default Ember.Mixin.create({
    notifyGoogleAnalytics: function() {
        if (!window.ga) { return; }
        return window.ga('send', 'pageview', {
            'page': this.get('url'),
            'title': this.get('url')
        });
    }.on('didTransition')
});

