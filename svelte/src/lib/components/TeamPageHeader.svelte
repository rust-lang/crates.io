<!--
  @component
  Renders the public team identity header.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import PageHeader from './PageHeader.svelte';
  import UserAvatar from './UserAvatar.svelte';

  interface TeamPageHeaderTeam {
    /** The team's avatar URL, if available. */
    avatar?: string | null;

    /** The team login in `github:organization:team` format. */
    login: string;

    /** The team name displayed below the organization. */
    name: string | null;

    /** The URL of the team's GitHub organization. */
    url: string | null;
  }

  interface Props {
    /** The public team identity displayed in the header. */
    team: TeamPageHeaderTeam;
  }

  let { team }: Props = $props();

  let orgName = $derived(team.login.split(':', 2)[1]);
</script>

<PageHeader style="display: flex; align-items: center;" data-test-heading>
  <UserAvatar
    user={{ avatar: team.avatar, kind: 'team', login: team.login, name: team.name }}
    size="medium"
    style="margin-right: var(--space-m)"
    data-test-avatar
  />
  <div>
    <div class="header-row">
      <h1 data-test-org-name>{orgName}</h1>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a href={team.url} title={team.login} class="github-link" data-test-github-link>
        <Icon class="i-simple-icons:github" label="GitHub profile" />
      </a>
    </div>
    <h2 data-test-team-name>{team.name}</h2>
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
</style>
