<!--
  @component
  Explains that a user's crates.io account is not linked to the GitHub account
  with the same username. Place it after the user's name inside the link to
  their user page, because the tooltip attaches to its parent element.
-->
<script lang="ts">
  import Tooltip from './Tooltip.svelte';

  interface Props {
    /** The crates.io username. */
    login: string;
  }

  let { login }: Props = $props();
</script>

<span class="sr-only">(not linked to GitHub user {login})</span>
<Tooltip>
  <strong>{login}</strong> on crates.io is not linked to <strong>{login}</strong> on GitHub.
</Tooltip>
