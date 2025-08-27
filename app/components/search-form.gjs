import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import onKey from 'ember-keyboard/helpers/on-key';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import eq from 'ember-truth-helpers/helpers/eq';

import focus from 'crates-io/helpers/focus';

export default class Header extends Component {
  @service header;
  @service router;

  @action updateSearchValue(event) {
    let { value } = event.target;
    this.header.searchValue = value;
  }

  @action search() {
    this.router.transitionTo('search', {
      queryParams: {
        q: this.header.searchValue,
        page: 1,
      },
    });
  }

  <template>
    <form
      action='/search'
      role='search'
      autocapitalize='off'
      autocomplete='off'
      autocorrect='off'
      spellcheck='false'
      data-test-search-form
      ...attributes
      class='form {{if (eq @size "big") "size-big"}}'
      {{on 'submit' (preventDefault this.search)}}
    >
      {{! template-lint-disable require-input-label}}
      {{! disabled due to https://github.com/ember-template-lint/ember-template-lint/issues/2141 }}

      {{! template-lint-disable no-autofocus-attribute}}
      {{! disabled because this is a "form field that serves as the main purpose of the page" }}
      {{! see https://github.com/ember-template-lint/ember-template-lint/blob/master/docs/rule/no-autofocus-attribute.md }}

      <input
        type='text'
        inputmode='search'
        class='input-lg'
        name='q'
        id='cargo-desktop-search'
        placeholder="Type 'S' or '/' to search"
        value={{this.header.searchValue}}
        oninput={{this.updateSearchValue}}
        autofocus={{@autofocus}}
        required
        aria-label='Search'
        data-test-search-input
      />

      {{! Second input fields for narrow screens because CSS does not allow us to change the placeholder }}
      <input
        type='text'
        inputmode='search'
        class='input-sm'
        name='q'
        placeholder='Search'
        value={{this.header.searchValue}}
        oninput={{this.updateSearchValue}}
        required
        aria-label='Search'
      />

      <button type='submit' class='submit-button button-reset'>
        <span class='sr-only'>Submit</span>
        {{svgJar 'search' class=(scopedClass 'submit-icon')}}
      </button>

      {{onKey 's' (focus '#cargo-desktop-search')}}
      {{onKey 'S' (focus '#cargo-desktop-search')}}
      {{onKey 'shift+s' (focus '#cargo-desktop-search')}}
      {{onKey '/' (focus '#cargo-desktop-search')}}
    </form>
  </template>
}
