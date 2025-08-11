import { LinkTo } from '@ember/routing';
<template>
  <@menu.Item ...attributes>
    <LinkTo @query={{@query}}>{{yield}}</LinkTo>
  </@menu.Item>
</template>
