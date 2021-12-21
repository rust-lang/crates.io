import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class MeIndexRoute extends Route {
  @service router;

  redirect() {
    // `cargo login` is showing links to https://crates.io/me to access the API tokens,
    // so we need to keep this route and redirect the user to the API tokens settings page.
    this.router.replaceWith('settings.tokens');
  }
}
