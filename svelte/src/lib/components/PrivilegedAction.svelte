<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import Tooltip from '#lib/components/Tooltip.svelte';
  import { getSession } from '#lib/utils/session.svelte.ts';

  /**
   * A component that wraps elements (probably mostly buttons in practice) that
   * can be used to perform potentially privileged actions.
   *
   * This component requires a `userAuthorised` property, which must be a
   * `boolean` indicating whether the user is authorised for this action. Admin
   * rights need not be taken into account.
   *
   * If the current user is an admin and they have enabled sudo mode, then they
   * are always allowed to perform the action, regardless of the return value of
   * `userAuthorised`.
   *
   * There are three content blocks supported by this component:
   *
   * - `children`: required; this is displayed when the user is authorised to
   *               perform the action.
   * - `placeholder`: this is displayed when the user _could_ be authorised to
   *                  perform the action (that is, they're an admin but have not
   *                  enabled sudo mode), but currently cannot perform the action.
   *                  If omitted, the `children` block is used with all form
   *                  controls disabled and a tooltip added.
   * - `unprivileged`: this is displayed when the user is not able to perform this
   *                   action, and cannot be authorised to do so. If omitted, an
   *                   empty block will be used.
   *
   * Note that all blocks will be output with a wrapping `<div>` for technical
   * reasons, so be sure to style accordingly if necessary.
   */
  interface Props extends Omit<HTMLAttributes<HTMLDivElement>, 'placeholder'> {
    userAuthorised: boolean;
    children: Snippet;
    placeholder?: Snippet;
    unprivileged?: Snippet;
  }

  let { userAuthorised, children, placeholder, unprivileged, ...others }: Props = $props();

  let session = getSession();
  let isPrivileged = $derived(userAuthorised || session.isSudoEnabled);
  let canBePrivileged = $derived(!userAuthorised && (session.currentUser?.is_admin ?? false) && !session.isSudoEnabled);
</script>

{#if isPrivileged}
  <div {...others}>
    {@render children()}
  </div>
{:else if canBePrivileged}
  {#if placeholder}
    <div {...others}>
      {@render placeholder()}
    </div>
  {:else}
    <div {...others} class="placeholder">
      <fieldset data-test-placeholder-fieldset disabled>
        {@render children()}
      </fieldset>
      <Tooltip>You must enable admin actions before you can perform this operation.</Tooltip>
    </div>
  {/if}
{:else}
  <div {...others}>
    {#if unprivileged}
      {@render unprivileged()}
    {/if}
  </div>
{/if}

<style>
  .placeholder {
    fieldset {
      border: 0;
      margin: 0;
      padding: 0;
    }

    fieldset[disabled] {
      cursor: not-allowed;

      :global([disabled]) {
        cursor: not-allowed;
      }

      /* This duplicates the styles in .button[disabled] as there's no
       * obvious way to compose them, given the target selectors. */
      :global(button),
      :global(.yellow-button),
      :global(.tan-button) {
        background: linear-gradient(to bottom, var(--bg-color-top-light) 0%, var(--bg-color-bottom-light) 100%);
        color: var(--disabled-text-color);
        cursor: not-allowed;
      }
    }
  }
</style>
