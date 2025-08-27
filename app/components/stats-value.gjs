import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

<template>
  <div ...attributes class='stats-value'>
    <span class='value' data-test-value>{{@value}}</span>
    <span class='label'>{{@label}}</span>
    {{svgJar @icon role='img' aria-hidden='true' class=(scopedClass 'icon')}}
  </div>
</template>
