<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import PendingOwnerInviteRow from '$lib/components/PendingOwnerInviteRow.svelte';

  let { data } = $props();
</script>

<PageTitle title="Pending Invites" />

<PageHeader title="Pending Owner Invites" />

<div class="list">
  {#if data.invites.length !== 0}
    <ul>
      {#each data.invites as invite (invite.crate_id)}
        <li>
          <PendingOwnerInviteRow {invite} />
        </li>
      {/each}
    </ul>
  {:else}
    <p data-test-empty-state>You don't seem to have any pending invitations.</p>
  {/if}
</div>

<style>
  .list {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    padding: var(--space-s);

    & + & {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
    }
  }

  p {
    padding: var(--space-s);
  }
</style>
