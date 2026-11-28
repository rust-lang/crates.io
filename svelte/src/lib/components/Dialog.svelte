<!--
  @component
  Modal dialog built on the native `<dialog>` element, with a title and a
  close button. Closing it via the button or Escape sets `open` to `false`
  and returns focus to the previously focused element. The page behind the
  dialog doesn't scroll while it is open.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Attachment } from 'svelte/attachments';

  import Icon from './Icon.svelte';
  import Portal from './portal/Portal.svelte';

  interface Props {
    open: boolean;
    title: string;
    /** Covers the whole viewport instead of centering the dialog. */
    fullscreen?: boolean;
    /** Button that opens the dialog via the browser's invoker commands, without a click handler. */
    invoker?: HTMLButtonElement;
    children: Snippet;
  }

  let { open = $bindable(), title, fullscreen = false, invoker, children }: Props = $props();

  let id = $props.id();

  let linkInvoker: Attachment<HTMLDialogElement> = dialog => {
    let button = invoker;
    if (!button) return;

    button.commandForElement = dialog;
    button.command = 'show-modal';
    return () => {
      button.commandForElement = null;
      button.command = '';
    };
  };

  let showModal: Attachment<HTMLDialogElement> = dialog => {
    if (!open) return;

    dialog.showModal();
    return () => dialog.close();
  };
</script>

<Portal>
  <dialog
    class={['dialog', { fullscreen }]}
    aria-labelledby="{id}-title"
    ontoggle={event => (open = event.newState === 'open')}
    {@attach linkInvoker}
    {@attach showModal}
  >
    {#if open}
      <div class="header">
        <h2 id="{id}-title" class="title">{title}</h2>
        <button type="button" class="close" aria-label="Close" onclick={() => (open = false)}>
          <Icon class="i-mdi:close" />
        </button>
      </div>

      {@render children()}
    {/if}
  </dialog>
</Portal>

<style>
  :global(html):has(.dialog:modal) {
    overflow: hidden;
  }

  .dialog {
    padding: 0;
    border: none;
    border-radius: var(--border-radius-small);
    background: var(--surface);
    color: var(--main-color);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.15);

    &[open] {
      display: flex;
      flex-direction: column;
    }

    &::backdrop {
      background: rgb(0 0 0 / 0.4);
    }
  }

  .fullscreen {
    width: 100vw;
    max-width: none;
    height: 100dvh;
    max-height: none;
    margin: 0;
    padding: env(safe-area-inset-top) 0 env(safe-area-inset-bottom);
    border-radius: 0;
    box-shadow: none;

    &::backdrop {
      background: var(--surface);
    }
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-xs);
    padding: var(--space-3xs) var(--space-3xs) 0 var(--space-xs);
  }

  .title {
    margin: 0;
    font-size: 1em;
    font-weight: 600;
  }

  .close {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    width: 44px;
    height: 44px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: none;
    color: var(--main-color);
    font-size: 24px;
    cursor: pointer;

    &:hover,
    &:focus-visible {
      background: var(--surface-hover);
    }
  }
</style>
