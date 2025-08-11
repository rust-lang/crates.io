import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';

// Six hours.
const SUDO_SESSION_DURATION_MS = 6 * 60 * 60 * 1000;

export default class Header extends Component {
  /** @type {import("../services/session").default} */
  @service session;

  @action
  enableSudo() {
    this.session.setSudo(SUDO_SESSION_DURATION_MS);
  }

  @action
  disableSudo() {
    this.session.setSudo(0);
  }
}
