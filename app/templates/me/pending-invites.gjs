import pageTitle from 'ember-page-title/helpers/page-title';

import PageHeader from 'crates-io/components/page-header';
import PendingOwnerInviteRow from 'crates-io/components/pending-owner-invite-row';
<template>
  {{pageTitle 'Pending Invites'}}

  <PageHeader @title='Pending Owner Invites' />

  <div class='list'>
    {{#each @controller.model as |invite|}}
      <PendingOwnerInviteRow @invite={{invite}} class='row' data-test-invite={{invite.crate_name}} />
    {{else}}
      <p class='row' data-test-empty-state>You don't seem to have any pending invitations.</p>
    {{/each}}
  </div>
</template>
