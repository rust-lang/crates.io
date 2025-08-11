import { service } from '@ember/service';
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

<Dropdown data-test-dark-mode-menu ...attributes class="dropdown" as |dd|>
  <dd.Trigger @hideArrow={{true}} class="trigger" data-test-dark-mode-toggle>
    {{svg-jar this.icon class=(scoped-class "icon")}}
    <span class="sr-only">Change color scheme</span>
  </dd.Trigger>

  <dd.Menu class="menu" as |menu|>
    {{#each this.colorSchemes as |colorScheme|}}
      <menu.Item>
        <button
          class="menu-button button-reset {{if (eq colorScheme.mode this.colorScheme.scheme) 'selected'}}"
          type="button"
          {{on 'click' (fn this.colorScheme.set colorScheme.mode)}}
        >
          {{svg-jar colorScheme.svg class=(scoped-class "icon")}} {{colorScheme.mode}}
        </button>
      </menu.Item>
    {{/each}}
  </dd.Menu>
</Dropdown>