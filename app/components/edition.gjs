import Tooltip from 'crates-io/components/tooltip';

<template>
  <span>
    {{@version.edition}}
    edition

    <Tooltip>
      This crate version does not declare a Minimum Supported Rust Version, but does require the
      {{@version.edition}}
      Rust Edition.

      <div class='edition-msrv'>
        {{@version.editionMsrv}}
        was the first version of Rust in this edition, but this crate may require features that were added in later
        versions of Rust.
      </div>
    </Tooltip>
  </span>
</template>
