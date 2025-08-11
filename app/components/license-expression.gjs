import parseLicense from 'crates-io/helpers/parse-license';
<template>
  {{#each (parseLicense @license) as |part|}}
    {{#if part.isKeyword}}
      <small>{{part.text}}</small>
    {{else if part.link}}
      <a href={{part.link}} rel='noreferrer'>
        {{part.text}}
      </a>
    {{else}}
      {{part.text}}
    {{/if}}
  {{/each}}
</template>
