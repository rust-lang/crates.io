<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import FollowButton from './FollowButton.svelte';

  const { Story } = defineMeta({
    title: 'FollowButton',
    component: FollowButton,
    tags: ['autodocs'],
    args: {
      onclick: () => {},
    },
  });
</script>

<Story name="Default" args={{ status: 'not-following' }} parameters={{ chromatic: { disableSnapshot: true } }} />

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Follow</h1>
  <FollowButton status="not-following" onclick={() => {}} />

  <h1>Following</h1>
  <FollowButton status="following" onclick={() => {}} />

  <h1>Loading</h1>
  <FollowButton status="loading" onclick={() => {}} />

  <h1>Disabled</h1>
  <FollowButton status="not-following" disabled onclick={() => {}} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
