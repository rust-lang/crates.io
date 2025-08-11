import { fn } from '@ember/helper';
import { on } from '@ember/modifier';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import eq from 'ember-truth-helpers/helpers/eq';

import Dropdown from 'crates-io/components/dropdown';

export default class Header extends Component {
  <template>
    <Dropdown data-test-dark-mode-menu ...attributes class='dropdown' as |dd|>
      <dd.Trigger @hideArrow={{true}} class='trigger' data-test-dark-mode-toggle>
        {{svgJar this.icon class=(scopedClass 'icon')}}
        <span class='sr-only'>Change color scheme</span>
      </dd.Trigger>

      <dd.Menu class='menu' as |menu|>
        {{#each this.colorSchemes as |colorScheme|}}
          <menu.Item>
            <button
              class='menu-button button-reset {{if (eq colorScheme.mode this.colorScheme.scheme) "selected"}}'
              type='button'
              {{on 'click' (fn this.colorScheme.set colorScheme.mode)}}
            >
              {{svgJar colorScheme.svg class=(scopedClass 'icon')}}
              {{colorScheme.mode}}
            </button>
          </menu.Item>
        {{/each}}
      </dd.Menu>
    </Dropdown>
  </template>
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
