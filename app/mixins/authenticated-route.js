import Mixin from '@ember/object/mixin';
import { inject as service } from '@ember/service';

export default Mixin.create({
    flashMessages: service(),
    session: service(),

    beforeModel(transition) {
        return this.get('session').checkCurrentUser(transition, () => {
            this.get('flashMessages').queue('Please log in to proceed');
        });
    },
});
