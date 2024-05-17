import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class Header extends Component {
  /** @type {import("../services/dark-mode").default} */
  @service colorScheme;

  colorSchemes = [
    { mode: 'light', svg: 'sun' },
    { mode: 'dark', svg: 'moon' },
    { mode: 'system', svg: 'color-mode' },
  ];

  get icon() {
    return this.colorSchemes.find(({ mode }) => mode === this.colorScheme.scheme)?.svg;
  }
}
