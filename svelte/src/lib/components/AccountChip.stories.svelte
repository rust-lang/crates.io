<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import AccountChip from './AccountChip.svelte';

  const { Story } = defineMeta({
    title: 'AccountChip',
    component: AccountChip,
    tags: ['autodocs'],
  });

  const LONG_HANDLE = 'averylonggithubhandlethatdoesnotfitintheavailablewidth';
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Matching</h1>
  <AccountChip provider="github" handle="tbieniek" href="https://github.com/tbieniek" />

  <h1>Mismatched</h1>
  <AccountChip provider="github" handle="Turbo87" href="https://github.com/Turbo87" mismatched />

  <h1>Long Handle</h1>
  <div class="constrained">
    <AccountChip provider="github" handle={LONG_HANDLE} href={`https://github.com/${LONG_HANDLE}`} />
  </div>

  <h1>Long Mismatched Handle</h1>
  <div class="constrained">
    <AccountChip provider="github" handle={LONG_HANDLE} href={`https://github.com/${LONG_HANDLE}`} mismatched />
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }

  .constrained {
    width: 220px;
    max-width: 100%;
  }
</style>
