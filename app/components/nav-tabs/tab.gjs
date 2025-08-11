import { on } from '@ember/modifier';
<template>
  <li ...attributes>
    <a
      href={{@link.url}}
      class='link {{if @link.isActive "active"}}'
      data-test-active={{@link.isActive}}
      {{on 'click' @link.transitionTo}}
    >
      {{yield}}
    </a>
  </li>
</template>
