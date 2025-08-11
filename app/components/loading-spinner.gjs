import eq from 'ember-truth-helpers/helpers/eq';
<template>
  <div ...attributes class='spinner {{if (eq @theme "light") "light"}}'>
    <span class='sr-only'>Loading…</span>
  </div>
</template>
