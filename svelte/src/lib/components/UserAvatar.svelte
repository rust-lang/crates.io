<script lang="ts">
  import type { HTMLImgAttributes } from 'svelte/elements';

  import avatarPlaceholder from '$lib/assets/avatar-placeholder.svg';

  type Size = 'small' | 'medium-small' | 'medium';

  interface AvatarUser {
    avatar?: string | null;
    kind: 'user' | 'team';
    login: string;
    name?: string | null;
  }

  interface Props extends Omit<HTMLImgAttributes, 'src' | 'width' | 'height' | 'alt'> {
    user: AvatarUser;
    size?: Size;
  }

  let { user, size = 'small', ...rest }: Props = $props();

  let sizeValue = $derived.by(() => {
    if (size === 'medium') return 85;
    if (size === 'medium-small') return 32;
    return 22;
  });

  let alt = $derived(user.name ? `${user.name} (${user.login})` : `(${user.login})`);

  let title = $derived(user.kind === 'team' ? `${user.name} team` : user.name);

  let src = $derived(user.avatar ? `${user.avatar}&s=${sizeValue * 2}` : avatarPlaceholder);
</script>

<img {src} width={sizeValue} height={sizeValue} {alt} {title} decoding="async" {...rest} />
