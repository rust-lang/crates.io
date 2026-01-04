<script lang="ts">
  import { browser } from '$app/environment';

  import CopyIcon from '$lib/assets/copy.svg?component';
  import CopyButton from '$lib/components/CopyButton.svelte';

  interface Props {
    crate: string;
    version: string;
    exactVersion?: boolean;
    binNames?: string[];
    hasLib?: boolean;
  }

  let { crate, version, exactVersion = false, binNames, hasLib }: Props = $props();

  let isClipboardSupported = browser && Boolean(navigator.clipboard?.writeText);

  let cargoInstallCommand = $derived(exactVersion ? `cargo install ${crate}@${version}` : `cargo install ${crate}`);

  let cargoAddCommand = $derived(exactVersion ? `cargo add ${crate}@=${version}` : `cargo add ${crate}`);

  let tomlSnippet = $derived.by(() => {
    let v = version.split('+')[0];
    let exact = exactVersion ? '=' : '';
    return `${crate} = "${exact}${v}"`;
  });
</script>

<div class="install-instructions">
  {#if binNames && binNames.length > 0}
    {#if isClipboardSupported}
      <CopyButton copyText={cargoInstallCommand} title="Copy command to clipboard" class="copy-button">
        <span class="selectable">{cargoInstallCommand}</span>
        <CopyIcon aria-hidden="true" class="copy-icon" />
      </CopyButton>
    {:else}
      <code class="copy-fallback">{cargoInstallCommand}</code>
    {/if}

    <p class="copy-help">
      {#if binNames.length === 1}
        Running the above command will globally install the <span class="bin-name">{binNames[0]}</span> binary.
      {:else if binNames.length === 2}
        Running the above command will globally install the <span class="bin-name">{binNames[0]}</span> and
        <span class="bin-name">{binNames[1]}</span> binaries.
      {:else}
        Running the above command will globally install these binaries:
        {#each binNames as binName, index (binName)}
          {#if index === 0}
            <span class="bin-name">{binName}</span>
          {:else if index === binNames.length - 1}
            , and <span class="bin-name">{binName}</span>
          {:else}
            , <span class="bin-name">{binName}</span>
          {/if}
        {/each}
      {/if}
    </p>
  {/if}

  {#if hasLib && binNames && binNames.length > 0}
    <h3>Install as library</h3>
  {/if}

  {#if hasLib}
    <p class="copy-help">Run the following Cargo command in your project directory:</p>

    {#if isClipboardSupported}
      <CopyButton copyText={cargoAddCommand} title="Copy command to clipboard" class="copy-button">
        <span class="selectable">{cargoAddCommand}</span>
        <CopyIcon aria-hidden="true" class="copy-icon" />
      </CopyButton>
    {:else}
      <code class="copy-fallback">{cargoAddCommand}</code>
    {/if}

    <p class="copy-help">Or add the following line to your Cargo.toml:</p>

    {#if isClipboardSupported}
      <CopyButton copyText={tomlSnippet} title="Copy Cargo.toml snippet to clipboard" class="copy-button">
        <span class="selectable">{tomlSnippet}</span>
        <CopyIcon aria-hidden="true" class="copy-icon" />
      </CopyButton>
    {:else}
      <code class="copy-fallback">{tomlSnippet}</code>
    {/if}
  {/if}
</div>

<style>
  .copy-help {
    font-size: 12px;
    overflow-wrap: break-word;

    &:last-child {
      margin-bottom: 0;
    }
  }

  .install-instructions :global(.copy-button),
  .copy-fallback {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2xs) var(--space-xs);
    font-family: var(--font-monospace);
    font-size: 14px;
    line-height: 1.5em;
    color: var(--main-color);
    background: transparent;
    border-radius: var(--space-3xs);
    border: solid var(--space-4xs) var(--gray-border);

    span {
      flex: auto;
      display: block;
      word-break: break-word;
    }
  }

  .install-instructions :global(.copy-button) {
    text-align: start;
    cursor: pointer;

    &:hover {
      background-color: light-dark(white, #141413);
    }
  }

  .install-instructions :global(.copy-button .copy-icon) {
    flex-shrink: 0;
    height: 1.1em;
    width: auto;
    margin-top: -3px;
    margin-left: var(--space-2xs);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .install-instructions :global(.copy-button:hover .copy-icon) {
    opacity: 1;
  }

  .selectable {
    user-select: text;
  }

  .bin-name {
    font-family: var(--font-monospace);
    font-weight: bold;
  }
</style>
