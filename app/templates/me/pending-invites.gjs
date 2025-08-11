{{page-title 'Pending Invites'}}

<PageHeader @title="Pending Owner Invites" />

<div class="list">
  {{#each this.model as |invite|}}
    <PendingOwnerInviteRow @invite={{invite}} class="row" data-test-invite={{invite.crate_name}} />
  {{else}}
    <p class="row" data-test-empty-state>You don't seem to have any pending invitations.</p>
  {{/each}}
</div>
