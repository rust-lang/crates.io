<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  // The icon is selected via the `class` prop rather than a `name` prop so
  // that UnoCSS can statically extract icon classes at build time. A
  // `name` prop would force the component to build the class string at
  // runtime (e.g. `i-${name}`), which defeats extraction.
  interface Props extends HTMLAttributes<HTMLSpanElement> {
    /**
     * UnoCSS icon class, e.g. `i-simple-icons:github`. The class must
     * appear as a literal string in source for static extraction to work.
     */
    class: string;

    /**
     * Accessible label for the icon. If set, the icon is exposed to
     * assistive technology as `role="img"` with this `aria-label`. If
     * omitted, the icon is decorative and gets `aria-hidden="true"`.
     */
    label?: string;
  }

  let { class: className, label, ...rest }: Props = $props();
</script>

<span
  class={['icon', className]}
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  {...rest}
></span>

<style>
  .icon {
    display: inline-block;
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }
</style>
