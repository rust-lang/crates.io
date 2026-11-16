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
  }

  const WIDTHS = [32, 64, 128, 256];
  const LARGEST_WIDTH = WIDTHS.at(-1);

  let { owner, class: className, ...rest }: Props = $props();

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
</span>

<style>
  .avatar {
    container-type: inline-size;
    display: inline-block;
    flex-shrink: 0;
    width: var(--avatar-size, 22px);
    height: var(--avatar-size, 22px);
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: max(1px, 3.5cqi);
    border-radius: 50%;
    background: white;
    box-shadow: var(--avatar-shadow, 1px 2px 2px 0 var(--surface-shadow-color-muted));
  }

  .team {
    border-radius: 4px;
  }
</style>
