import Route from '@ember/routing/route';
import RSVP from 'rsvp';

import AuthenticatedRoute from '../mixins/authenticated-route';

export default Route.extend(AuthenticatedRoute, {
    data: {},

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('myCrates', this.get('data.myCrates'));
        controller.set('myFollowing', this.get('data.myFollowing'));
        controller.set('myStats', this.get('data.myStats'));

        if (!controller.get('loadingMore')) {
            controller.set('myFeed', []);
            controller.send('loadMore');
        }
    },

    model() {
        return this.session.get('currentUser');
    },

    afterModel(user) {
        let myCrates = this.store.query('crate', {
            user_id: user.get('id')
        });

        let myFollowing = this.store.query('crate', {
            following: 1
        });

        let myStats = user.stats();

        return RSVP.hash({
            myCrates,
            myFollowing,
            myStats
        }).then((hash) => {
            this.set('data', hash);
        });
    }
});
