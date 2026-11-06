<!--
  @component
  Renders the public organization identity header.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import PageHeader from './PageHeader.svelte';
  import UserAvatar from './UserAvatar.svelte';
  import { resolve } from '$app/paths';
  import type { components } from '@crates-io/api-client';

  type Team = components["schemas"]["Team"];

  interface OrgPageHeaderOrg {
    /** The teams in this org in the crates.io database. */
    teams: Team[],
  }

  interface Props {
    /** The org login in `github:organization` format. */
    login: string;

    /** The public org identity displayed in the header. */
    org: OrgPageHeaderOrg;
  }

  let { org, login }: Props = $props();

  let avatar = $derived(org.teams[0]?.avatar);
  let url = $derived(org.teams[0]?.url);

</script>

<PageHeader style="display: flex; align-items: center;" data-test-heading>
  <UserAvatar
    user={{ avatar: avatar, kind: 'team', login: login, name: login }}
    size="medium"
    class="org-page-avatar me-m"
    data-test-avatar
  />
  <div>
    <div class="header-row">
      <h1 data-test-org-name>{login}</h1>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a href={url} title={login} class="github-link" data-test-github-link>
        <Icon class="i-simple-icons:github" label="GitHub profile" />
      </a>
    </div>
    {#if org.teams.length === 0 }
      <h2 data-test-org-teams>no teams found</h2>
    {:else}
      <h2 data-test-org-teams>teams:</h2>
      <ul class="org-teams">
        {#each org.teams as team (team.login)}
          <li>
            <a href="{resolve('/teams/[team_id]', { team_id: team.login })}">{team.name}</a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</PageHeader>

<style>
  h1,
  h2 {
    margin: 0;
    padding: 0;
  }

  h2 {
    margin-top: var(--space-2xs);
    color: var(--main-color-light);
  }

  :global(.team-page-avatar) {
    border-radius: 4px;
    object-fit: cover;
    background: white;
    padding: 3px;
    box-shadow: 1px 2px 2px 0 var(--surface-shadow-color-muted);
  }

  .header-row {
    display: flex;
    align-items: center;
  }

  .github-link {
    margin-left: var(--space-s);
    --icon-size: 32px;

    &,
    &:hover {
      color: var(--main-color);
    }
  }

  .org-teams {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2xs);
    margin: var(--space-xs) 0 0;
    padding: 0;
    font-size: calc(0.85 * var(--space-s));
    overflow: hidden;
  }
</style>
