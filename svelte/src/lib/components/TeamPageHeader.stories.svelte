<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TeamPageHeader from './TeamPageHeader.svelte';

  type Team = ComponentProps<typeof TeamPageHeader>['team'];

  const { Story } = defineMeta({
    title: 'TeamPageHeader',
    component: TeamPageHeader,
    tags: ['autodocs'],
  });

  const TEAM: Team = {
    avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
    login: 'github:rust-lang:libs',
    name: 'libs',
    url: 'https://github.com/rust-lang',
  };

  const TEAM_WITHOUT_AVATAR: Team = {
    ...TEAM,
    avatar: null,
  };

  const TEAM_WITH_LONG_IDENTITY: Team = {
    ...TEAM,
    login: 'github:averylonggithuborganizationnamethatcannotfitintheavailableheaderwidth:maintainers',
    name: 'A Very Long Team Name That Exceeds the Available Header Width',
  };
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Default</h1>
  <TeamPageHeader team={TEAM} />

  <h1>Without Avatar</h1>
  <TeamPageHeader team={TEAM_WITHOUT_AVATAR} />

  <h1>Long Identity</h1>
  <div class="long-identity">
    <TeamPageHeader team={TEAM_WITH_LONG_IDENTITY} />
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }

  .long-identity {
    width: 500px;
    max-width: 100%;
  }
</style>
