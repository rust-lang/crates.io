<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import PendingOwnerInviteRow from '$lib/components/PendingOwnerInviteRow.svelte';

  let { data } = $props();
</script>

<PageTitle title="Pending Invites" />

<PageHeader title="Pending Owner Invites" />

<div class="list">
  {#each data.invites as invite (invite.crate_id)}
    <PendingOwnerInviteRow {invite} />
  {:else}
    <p data-test-empty-state>You don't seem to have any pending invitations.</p>
  {/each}
</div>

<style>
  .list {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);

    > :global(*) {
      padding: var(--space-s);
    }

    > :global(* + *) {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
    }
  }
</style>
