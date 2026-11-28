<script module lang="ts">
  import { resolve } from '$app/paths';
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import VersionSwitcher from './VersionSwitcher.svelte';

  const { Story } = defineMeta({
    title: 'version-switcher/VersionSwitcher',
    component: VersionSwitcher,
    parameters: { controls: { disable: true } },
  });

  const STORE = {
    versions: Array.from({ length: 30 }, (_, i) => ({
      num: `1.0.${29 - i}`,
      created_at: `2026-05-${String(30 - i).padStart(2, '0')}T12:00:00Z`,
      yanked: i % 7 === 3,
    })),
    isComplete: true,
    error: false,
    loadMore: () => Promise.resolve(),
  };

  function versionHref(num: string) {
    return resolve('/crates/[crate_id]/[version_num]', { crate_id: 'serde', version_num: num });
  }
</script>

<!-- Opens the popover, so the visual regression snapshot shows the panel instead of only the button -->
<Story
  name="Default"
  asChild
  play={async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole('button', { name: 'v1.0.20' }));
  }}
>
  <h1 class="header">
    serde
    <VersionSwitcher store={STORE} crateName="serde" currentNum="1.0.20" defaultNum="1.0.29" {versionHref} />
  </h1>
</Story>

<style>
  .header {
    height: 560px;
  }
</style>
