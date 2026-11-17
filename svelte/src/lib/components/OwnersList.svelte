<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import Avatar from './Avatar.svelte';

  type Owner = components['schemas']['Owner'];

  interface Props {
    owners: Owner[];
  }

  let { owners }: Props = $props();

  let showDetailedList = $derived(owners.length <= 5);

  function displayName(owner: Owner): string {
    if (owner.kind === 'team') {
      // For teams, compute display_name as org_name/name
      // login format is "github:org_name:team_name"
      let orgName = owner.login.split(':', 2)[1];
      return owner.name ? `${orgName}/${owner.name}` : owner.login;
    }
    return owner.name ?? owner.login;
  }
</script>

<ul
  role="list"
  class="list"
  class:detailed={showDetailedList}
  data-test-owners={showDetailedList ? 'detailed' : 'basic'}
>
  {#each owners as owner (`${owner.kind}:${owner.id}`)}
    {@const isTeam = owner.kind === 'team'}
    {@const href = isTeam
      ? resolve('/teams/[team_id]', { team_id: owner.login })
      : resolve('/users/[user_id]', { user_id: owner.login })}
    <li>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() is used above -->
      <a {href} class="link" data-test-owner-link={owner.login}>
        <Avatar {owner} aria-hidden="true" />
        <span class="name" class:sr-only={!showDetailedList}>{displayName(owner)}</span>
      </a>
    </li>
  {/each}
</ul>

<style>
  .list.detailed {
    list-style: none;
    padding: 0;
    margin: 0;

    > * + * {
      margin-top: 5px;
    }

    .link {
      display: grid;
      grid-template-columns: auto 1fr;
      column-gap: 10px;
      align-items: center;
    }

    .name {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }

  .list:not(.detailed) {
    display: flex;
    flex-wrap: wrap;
    list-style: none;
    padding: 0;
    margin: 0 0 -10px;

    > * {
      margin: 0 10px 10px 0;
    }
  }

  .link {
    --avatar-size: 32px;
  }
</style>
