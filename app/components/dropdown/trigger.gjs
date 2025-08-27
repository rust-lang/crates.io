import { on } from '@ember/modifier';

<template>
  <button type='button' {{on 'click' @toggle}} ...attributes class='button'>
    {{yield}}
    {{#unless @hideArrow}}
      <span class='arrow'></span>
    {{/unless}}
  </button>
</template>
