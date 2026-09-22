<!--
  @component
  Renders a keyword as a link to its keyword page.
-->
<script lang="ts">
  import { resolve } from '$app/paths';

  interface Props {
    /** The keyword identifier displayed by and linked from the pill. */
    keyword: string;
  }

  let { keyword }: Props = $props();
</script>

<a href={resolve('/keywords/[keyword_id]', { keyword_id: keyword })} data-test-keyword={keyword}>
  <span class="hash" aria-hidden="true">#</span>{keyword}
</a>

<style>
  a {
    display: inline-flex;
    align-items: center;
    gap: var(--space-4xs);
    padding: var(--space-4xs) var(--space-xs);
    color: var(--main-color-light);
    background: var(--main-bg);
    border-radius: 99999px;
    white-space: nowrap;
    transition: color var(--transition-fast);

    &:hover {
      color: var(--main-color);
    }
  }

  .hash {
    font-family: var(--font-monospace);
    color: var(--main-color-light);
    opacity: 0.65;
  }
</style>
