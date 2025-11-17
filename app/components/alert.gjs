import svgJar from 'ember-svg-jar/helpers/svg-jar';
import { eq } from 'ember-truth-helpers';

<template>
  <div ...attributes class='alert' data-variant={{@variant}}>
    {{#if (eq @variant 'warning')}}
      {{svgJar 'triangle-exclamation'}}
    {{/if}}
    <div class='alert-content'>
      {{yield}}
    </div>
  </div>
</template>
