import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class CrateDocsRoute extends Route {
  @service notifications;
  @service redirector;
  @service router;

  redirect() {
    let crate = this.modelFor('crate');

    let documentation = crate.get('documentation');
    if (documentation) {
      this.redirector.redirectTo(documentation);
    } else {
      // Redirect to the crate's main page and show a flash error if
      // no documentation is found
      let message = 'Crate does not supply a documentation URL';
      this.notifications.error(message);
      this.router.replaceWith('crate', crate);
    }
  }
}
