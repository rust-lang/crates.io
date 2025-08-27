import { hash } from '@ember/helper';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import Dropdown from 'crates-io/components/dropdown';
import SortDropdownOption from 'crates-io/components/sort-dropdown/option';

<template>
  <Dropdown as |dd|>
    <dd.Trigger class='trigger' data-test-current-order>
      {{svgJar 'sort' class=(scopedClass 'icon')}}
      {{@current}}
    </dd.Trigger>

    <dd.Menu as |menu|>
      {{yield (hash Option=(component SortDropdownOption menu=menu))}}
    </dd.Menu>
  </Dropdown>
</template>
