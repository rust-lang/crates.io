<script module lang="ts">
  import { resolve } from '$app/paths';
  import { defineMeta } from '@storybook/addon-svelte-csf';
  import { action } from 'storybook/actions';

  import Panel from './Panel.svelte';

  const { Story } = defineMeta({
    title: 'version-switcher/Panel',
    component: Panel,
    parameters: { controls: { disable: true } },
  });

  const VERSIONS = ['1.3.0', '1.2.3', '1.2.2', '1.2.1', '1.2.0', '1.1.0', '1.0.0'].map((num, i) => ({
    num,
    created_at: `2026-05-0${7 - i}T12:00:00Z`,
    yanked: num === '1.2.1',
  }));

  function versionHref(num: string) {
    return resolve('/crates/[crate_id]/[version_num]', { crate_id: 'serde', version_num: num });
  }

  const STATES = [
    { isComplete: true, error: false },
    { isComplete: false, error: false },
    { isComplete: false, error: true },
  ];

  const onLoadMore = action('onLoadMore');
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Default" asChild>
  <div class="examples">
    {#each STATES as state, i (i)}
      <div class="frame">
        <Panel versions={VERSIONS} {...state} currentNum="1.2.3" defaultNum="1.3.0" {versionHref} {onLoadMore} />
      </div>
    {/each}
  </div>
</Story>

<style>
  .examples {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-m);
  }

  .frame {
    display: flex;
    flex-direction: column;
    width: 360px;
    height: 240px;
    background: var(--surface);
  }
</style>
