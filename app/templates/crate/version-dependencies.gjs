import pageTitle from 'ember-page-title/helpers/page-title';

import CrateHeader from 'crates-io/components/crate-header';
import Row from 'crates-io/components/dependency-list/row';

<template>
  {{pageTitle @controller.crate.name}}

  <CrateHeader @crate={{@controller.crate}} @version={{@controller.version}} @versionNum={{@controller.version.num}} />

  <h2 class='heading'>Dependencies</h2>
  {{#if @controller.version.normalDependencies}}
    <ul class='list' data-test-dependencies>
      {{#each @controller.version.normalDependencies as |dependency|}}
        <li><Row @dependency={{dependency}} /></li>
      {{/each}}
    </ul>
  {{else}}
    <div class='no-deps' data-test-no-dependencies>
      This version of the "{{@controller.crate.name}}" crate has no dependencies
    </div>
  {{/if}}

  {{#if @controller.version.buildDependencies}}
    <h2 class='heading'>Build-Dependencies</h2>
    <ul class='list' data-test-build-dependencies>
      {{#each @controller.version.buildDependencies as |dependency|}}
        <li><Row @dependency={{dependency}} /></li>
      {{/each}}
    </ul>
  {{/if}}

  {{#if @controller.version.devDependencies}}
    <h2 class='heading'>Dev-Dependencies</h2>
    <ul class='list' data-test-dev-dependencies>
      {{#each @controller.version.devDependencies as |dependency|}}
        <li><Row @dependency={{dependency}} /></li>
      {{/each}}
    </ul>
  {{/if}}
</template>
