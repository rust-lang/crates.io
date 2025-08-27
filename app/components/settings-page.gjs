import link_ from 'ember-link/helpers/link';

import SideMenu from 'crates-io/components/side-menu';

<template>
  <div ...attributes class='page'>
    <SideMenu data-test-settings-menu as |menu|>
      <menu.Item @link={{link_ 'settings.profile'}}>Profile</menu.Item>
      <menu.Item @link={{link_ 'settings.tokens'}} data-test-tokens>API Tokens</menu.Item>
    </SideMenu>

    <div class='content'>
      {{yield}}
    </div>
  </div>
</template>
