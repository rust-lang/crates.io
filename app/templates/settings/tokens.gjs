import pageTitle from 'ember-page-title/helpers/page-title';

import PageHeader from 'crates-io/components/page-header';
import SettingsPage from 'crates-io/components/settings-page';
<template>
  {{pageTitle 'Settings'}}

  <PageHeader @title='Account Settings' />

  <SettingsPage>
    {{outlet}}
  </SettingsPage>
</template>
