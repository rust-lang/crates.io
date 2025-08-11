import { on } from '@ember/modifier';
<template>
  <li ...attributes>
    <a href={{@link.url}} class='link {{if @link.isActive "active"}}' {{on 'click' @link.transitionTo}}>{{yield}}</a>
  </li>
</template>
