import { on } from '@ember/modifier';

import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import pageTitle from 'ember-page-title/helpers/page-title';

import OwnedCrateRow from 'crates-io/components/owned-crate-row';
import PageHeader from 'crates-io/components/page-header';
import SettingsPage from 'crates-io/components/settings-page';

<template>
  {{pageTitle 'Settings'}}

  <PageHeader @title='Account Settings' />

  <SettingsPage>
    <form class='me-email-notifications' {{on 'submit' (preventDefault @controller.saveEmailNotifications)}}>
      <h2>Email Notification Preferences</h2>

      {{#if @controller.hasEmailNotificationFeature}}
        <p>
          To aid detection of unauthorized crate changes, we email you each time a new version of a crate you own is
          pushed. By receiving and reading these emails, you help protect the Rust ecosystem. You may also choose to
          turn these emails off for any of your crates listed below.
        </p>

        <div class='notifications-row'>
          <button
            type='button'
            class='button button--small'
            {{on 'click' @controller.emailNotificationsSelectAll}}
          >Select All</button>
          <button
            type='button'
            class='button button--small'
            {{on 'click' @controller.emailNotificationsSelectNone}}
          >Deselect All</button>
        </div>

        <ul class='notifications-list'>
          {{#each @controller.ownedCrates as |ownedCrate|}}
            <li>
              <OwnedCrateRow @ownedCrate={{ownedCrate}} />
            </li>
          {{/each}}
        </ul>

        <div class='notifications-row'>
          {{#if @controller.emailNotificationsError}}
            <div class='notifications-error'>
              An error occurred while saving your email preferences.
            </div>
          {{/if}}
          {{#if @controller.emailNotificationsSuccess}}
            <div class='notifications-success'>
              Your email notification preferences have been updated!
            </div>
          {{/if}}
          <div class='right'>
            <button type='submit' class='button'>Update</button>
          </div>
        </div>
      {{else}}
        <p>
          To aid detection of unauthorized crate changes, we plan to email you each time a new version of a crate you
          own is pushed. This feature is still work-in-progress, if you want to help out have a look at
          <a href='https://github.com/rust-lang/crates.io/issues/1895'>#1895</a>.
        </p>
      {{/if}}
    </form>
  </SettingsPage>
</template>
