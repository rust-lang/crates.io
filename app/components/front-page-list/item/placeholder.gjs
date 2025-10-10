import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import Placeholder from 'crates-io/components/placeholder';

<template>
  <div ...attributes class='link'>
    <div class='left'>
      <Placeholder class='title' />
      {{#if @withSubtitle}}<Placeholder class='subtitle' />{{/if}}
    </div>
    {{svgJar 'chevron-right' class=(scopedClass 'right')}}
  </div>
</template>
