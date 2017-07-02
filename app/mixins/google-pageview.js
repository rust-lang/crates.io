import Ember from 'ember';

export default Ember.Mixin.create({

    metrics: Ember.inject.service(),

    notifyGoogleAnalytics: Ember.on('didTransition', function() {
        Ember.run.scheduleOnce('afterRender', this, () => {
            const page = this.get('url');
            const title = this.get('url');
            Ember.get(this, 'metrics').trackPage({ page, title });
        });
    })
});

