import Controller from '@ember/controller';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

export default class SecurityController extends Controller {
  @service releaseTracks;
  @service sentry;

  @tracked crate;
  @tracked data;

  constructor() {
    super(...arguments);
    this.reset();
  }

  reset() {
    this.crate = undefined;
    this.data = undefined;
  }
}
