import { Input } from '@ember/component';
import { on } from '@ember/modifier';

import perform from 'ember-concurrency/helpers/perform';
import pageTitle from 'ember-page-title/helpers/page-title';

import EmailInput from 'crates-io/components/email-input';
import LoadingSpinner from 'crates-io/components/loading-spinner';
import PageHeader from 'crates-io/components/page-header';
import SettingsPage from 'crates-io/components/settings-page';
import UserAvatar from 'crates-io/components/user-avatar';
import UserLink from 'crates-io/components/user-link';

<template>
  {{pageTitle 'Settings'}}

  <PageHeader @title='Account Settings' />

  <SettingsPage>
    <div class='me-profile'>
      <h2>Profile Information</h2>

      <div class='info'>
        <UserLink @user={{@controller.model.user}}>
          <UserAvatar @user={{@controller.model.user}} @size='medium' />
        </UserLink>

        <dl>
          <dt>Name</dt>
          <dd>{{@controller.model.user.name}}</dd>
          <dt>GitHub Account</dt>
          <dd>{{@controller.model.user.login}}</dd>
        </dl>
      </div>

      <p>
        To update your name and GitHub account, change them in your GitHub profile, then sign out and login again to
        crates.io. You cannot change these settings directly on crates.io, but we accept whatever values come from
        GitHub.
      </p>
    </div>

    <div class='me-email'>
      <h2>User Email</h2>
      <EmailInput @user={{@controller.model.user}} data-test-email-input />
    </div>

    <div class='notifications' data-test-notifications>
      <h2>Notification Settings</h2>

      <label class='checkbox-input'>
        <Input
          @type='checkbox'
          @checked={{@controller.publishNotifications}}
          disabled={{@controller.updateNotificationSettings.isRunning}}
          {{on 'change' @controller.handleNotificationsChange}}
        />
        <span class='label'>Publish Notifications</span>
        <span class='note'>
          Publish notifications are sent to your email address whenever new versions of a crate that you own are
          published. These can be useful to quickly detect compromised accounts or API tokens.
        </span>
      </label>

      <div class='buttons'>
        <button
          type='button'
          class='button button--small'
          disabled={{@controller.updateNotificationSettings.isRunning}}
          {{on 'click' (perform @controller.updateNotificationSettings)}}
        >
          Update preferences
        </button>
        {{#if @controller.updateNotificationSettings.isRunning}}
          <LoadingSpinner class='spinner' data-test-spinner />
        {{/if}}
      </div>
    </div>
  </SettingsPage>
</template>
