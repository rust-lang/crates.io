import pageTitle from 'ember-page-title/helpers/page-title';

import PageHeader from 'crates-io/components/page-header';
<template>
  {{pageTitle 'Category Slugs'}}

  <PageHeader @title='All Valid Category Slugs' />

  <dl class='list'>
    {{#each @controller.model as |category|}}
      <dt data-test-category-slug={{category.slug}}>{{category.slug}}</dt>
      <dd data-test-category-description={{category.slug}}>{{category.description}}</dd>
    {{/each}}
  </dl>
</template>
