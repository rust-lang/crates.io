import CrateRow from 'crates-io/components/crate-row';
<template>
  <div ...attributes>
    {{! The extra div wrapper is needed for specificity issues with `margin` }}
    <ol class='list'>
      {{#each @crates as |crate index|}}
        <li>
          <CrateRow @crate={{crate}} data-test-crate-row={{index}} />
        </li>
      {{/each}}
    </ol>
  </div>
</template>
