<script module lang="ts">
  import { resolve } from '$app/paths';
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Row from './Row.svelte';

  const { Story } = defineMeta({
    title: 'version-switcher/Row',
    component: Row,
    parameters: { controls: { disable: true } },
  });

  function href(num: string) {
    return resolve('/crates/[crate_id]/[version_num]', { crate_id: 'serde', version_num: num });
  }

  function createVersion(num: string, yanked = false) {
    return { num, created_at: '2026-05-04T12:00:00Z', yanked };
  }

  const LONG_NUM = '2.0.0-alpha.1+build.2026-05-04.very-long-metadata';
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Default" asChild>
  <div class="list">
    <Row version={createVersion('1.3.0')} href={href('1.3.0')} />
    <Row version={createVersion('1.2.3')} href={href('1.2.3')} isCurrent />
    <Row version={createVersion('1.2.2')} href={href('1.2.2')} isDefault />
    <Row version={createVersion('1.2.1')} href={href('1.2.1')} isCurrent isDefault />
    <Row version={createVersion('1.2.0', true)} href={href('1.2.0')} />
    <Row version={createVersion('1.1.0', true)} href={href('1.1.0')} isCurrent />
    <Row version={createVersion(LONG_NUM)} href={href(LONG_NUM)} isDefault />
  </div>
</Story>

<style>
  .list {
    display: flex;
    flex-direction: column;
    width: 360px;
    background: var(--surface);
  }
</style>
