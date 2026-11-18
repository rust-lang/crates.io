<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import avatarPlaceholder from '#lib/assets/avatar-placeholder.svg';

  interface AvatarOwner {
    avatar?: string | null;
    kind: 'user' | 'team';
    login: string;
    name?: string | null;
  }

  interface Props extends HTMLAttributes<HTMLSpanElement> {
    owner: AvatarOwner;

    /** Whether the crates.io username differs from the linked GitHub username. */
    mismatched?: boolean;
  }

  const WIDTHS = [32, 64, 128, 256];
  const LARGEST_WIDTH = WIDTHS.at(-1);

  let { owner, mismatched = false, class: className, ...rest }: Props = $props();

  let alt = $derived(owner.name ? `${owner.name} (${owner.login})` : `(${owner.login})`);

  let title = $derived(owner.kind === 'team' ? `${owner.name} team` : owner.name);

  let srcset = $derived(owner.avatar ? WIDTHS.map(width => `${owner.avatar}&s=${width} ${width}w`).join(', ') : null);
</script>

<span class={['avatar', className]} {...rest}>
  <img
    src={owner.avatar ? `${owner.avatar}&s=${LARGEST_WIDTH}` : avatarPlaceholder}
    {srcset}
    sizes={srcset ? 'auto' : null}
    loading="lazy"
    {alt}
    {title}
    decoding="async"
    class={{ team: owner.kind === 'team' }}
  />
  {#if mismatched}
    <span class="mismatch-badge" aria-hidden="true" data-test-mismatch-badge>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.75">
        <path d="M6 9.75h12M6 14.25h12M14.75 5.5l-5.5 13" />
      </svg>
    </span>
  {/if}
</span>

<style>
  .avatar {
    --padding: max(1px, 3.5cqi);

    container-type: inline-size;
    position: relative;
    display: inline-block;
    flex-shrink: 0;
    filter: drop-shadow(var(--avatar-shadow, 1px 2px 2px var(--surface-shadow-color-muted)));
    width: var(--avatar-size, 22px);
    height: var(--avatar-size, 22px);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: var(--padding);
    border-radius: 50%;
    background: white;
  }

  .team {
    border-radius: 4px;
  }

  .mismatch-badge {
    --size: max(44cqi, 9px + 20cqi);
    --offset: calc(14cqi - var(--size) / 2);
    position: absolute;
    right: var(--offset);
    bottom: var(--offset);
    width: var(--size);
    height: var(--size);
    padding: var(--padding);
    border-radius: 50%;
    background:
      linear-gradient(var(--orange-700)) content-box,
      white;
    color: white;

    svg {
      display: block;
      width: 100%;
      height: 100%;
    }
  }
</style>
