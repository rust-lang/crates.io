<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Panel from '$lib/components/Panel.svelte';
  import PendingOwnerInviteRow from '$lib/components/PendingOwnerInviteRow.svelte';

  let { data } = $props();
</script>

<PageTitle title="Pending Invites" />

<PageHeader title="Pending Owner Invites" />

<Panel class="mb-s">
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
</Panel>

<style>
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
