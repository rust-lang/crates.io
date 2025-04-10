import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';

export default class CrateRoute extends Route {
  @service headData;
  @service router;
  @service store;

  async model(params, transition) {
    let crateName = params.crate_id;

    try {
      // We would like the peeked crate to include information (such as keywords) for further
      // processing. Currently, we determine this by checking if associated versions exist,
      // as default_version is included in the queryRecord call.
      // See: https://github.com/rust-lang/crates.io/issues/10663
      let crate = this.store.peekRecord('crate', crateName);
      if (!crate || crate.hasMany('versions').value() == null) {
        crate = await this.store.queryRecord('crate', { name: crateName });
      }
      return crate;
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${crateName}: Crate not found`;
        this.router.replaceWith('catch-all', { transition, error, title });
      } else {
        let title = `${crateName}: Failed to load crate data`;
        this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }
  }

  setupController(controller, model) {
    super.setupController(...arguments);
    this.headData.crate = model;
    waitForPromise(model.loadOwnerUserTask.perform()).catch(() => {
      // ignore all errors if the request fails
    });
  }

  resetController() {
    super.resetController(...arguments);
    this.headData.crate = null;
  }
}
