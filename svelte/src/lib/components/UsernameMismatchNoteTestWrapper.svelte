<script lang="ts">
  import UsernameMismatchNote from './UsernameMismatchNote.svelte';

  interface Props {
    name: string;
    login: string;
  }

  let { name, login }: Props = $props();
</script>

<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
<a href="/users/{login}">{name} <UsernameMismatchNote {login} /></a>
