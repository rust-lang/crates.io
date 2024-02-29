import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class Header extends Component {
  /** @type {import("../services/session").default} */
  @service session;

  @action
  enableSudo() {
    // FIXME: hard coded six hour duration.
    this.session.setSudo(6 * 60 * 60 * 1000);
  }

  @action
  disableSudo() {
    this.session.setSudo(0);
  }
}
