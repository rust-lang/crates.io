import { on } from '@ember/modifier';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
<template>
  <a href={{@link.url}} ...attributes class='box' {{on 'click' @link.transitionTo}}>
    <div class='left'>
      <div class='title'>{{@title}}</div>
      {{#if @subtitle}}<div class='subtitle'>{{@subtitle}}</div>{{/if}}
    </div>
    {{svgJar 'chevron-right' class=(scopedClass 'right')}}
  </a>
</template>
