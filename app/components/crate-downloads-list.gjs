import { LinkTo } from '@ember/routing';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import formatNum from 'crates-io/helpers/format-num';
<template>
  <ul class='list'>
    {{#each @crates as |crate|}}
      <li>
        <LinkTo @route='crate' @model={{crate.id}} class='link'>
          {{crate.name}}
          ({{crate.max_version}})
          {{svgJar 'download-arrow' class=(scopedClass 'download-icon')}}
          {{formatNum crate.downloads}}
        </LinkTo>
      </li>
    {{/each}}
  </ul>
</template>
