import { hash } from '@ember/helper';
import { LinkTo } from '@ember/routing';

import eq from 'ember-truth-helpers/helpers/eq';

import PageHeader from 'crates-io/components/page-header';
import CrateReportForm from 'crates-io/components/support/crate-report-form';
import TextContent from 'crates-io/components/text-content';

<template>
  <PageHeader @title='Contact Us' />

  <TextContent data-test-id='support-main-content'>
    {{#if @controller.supported}}
      {{#if (eq @controller.inquire 'crate-violation')}}
        <section data-test-id='crate-violation-section'>
          <CrateReportForm @crate={{@controller.crate}} />
        </section>
      {{/if}}
    {{else}}
      <section data-test-id='inquire-list-section'>
        <h2>Choose one of the these categories to continue.</h2>
        <ul class='inquire-list' data-test-id='inquire-list'>
          {{#each @controller.supports as |support|}}
            <li>
              <LinkTo
                @route='support'
                @query={{hash inquire=support.inquire}}
                data-test-id='link-{{support.inquire}}'
                class='link box-link'
              >
                {{support.label}}
              </LinkTo>
            </li>
          {{/each}}
          <li>
            <a href='mailto:help@crates.io' data-test-id='link-email-support' class='link box-link'>
              For all other cases:
              <strong>help@crates.io</strong>
            </a>
          </li>
        </ul>
      </section>
    {{/if}}
  </TextContent>
</template>
