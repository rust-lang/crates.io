import CrateVersionPage from 'crates-io/components/crate-version-page';

<template>
  <CrateVersionPage @crate={{@model.crate}} @version={{@model.version}} @requestedVersion={{@model.requestedVersion}} />
</template>
