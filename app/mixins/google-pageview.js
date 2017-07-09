import Mixin from '@ember/object/mixin';
import { get } from '@ember/object';
import { scheduleOnce } from '@ember/runloop';
import { on } from '@ember/object/evented';
import { inject as service } from '@ember/service';

export default Mixin.create({

    metrics: service(),

    notifyGoogleAnalytics: on('didTransition', function() {
        scheduleOnce('afterRender', this, () => {
            const page = this.get('url');
            const title = this.get('url');
            get(this, 'metrics').trackPage({ page, title });
        });
    })
});

