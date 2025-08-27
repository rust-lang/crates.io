import Tooltip from 'crates-io/components/tooltip';

<template>
  <span>
    v{{@version.msrv}}

    <Tooltip>
      &quot;Minimum Supported Rust Version&quot;
      {{#if @version.edition}}
        <div class='edition'>requires Rust Edition {{@version.edition}}</div>
      {{/if}}
    </Tooltip>
  </span>
</template>
