<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { HTMLAttributes } from 'svelte/elements';

  import { resolve } from '$app/paths';

  import DownloadArrowIcon from '$lib/assets/download-arrow.svg?component';
  import Placeholder from '$lib/components/Placeholder.svelte';

  type Dependency = components['schemas']['Dependency'] & {
    dependentCrateName: string;
  };

  interface Props extends HTMLAttributes<HTMLDivElement> {
    dependency: Dependency;
    descriptionPromise: Promise<string | null> | undefined;
  }

  let { dependency, descriptionPromise, ...restProps }: Props = $props();

  let focused = $state(false);

  const numberFormat = new Intl.NumberFormat();
</script>

<div {...restProps} class="row" class:focused>
  <div class="top">
    <div class="left">
      <a
        href={resolve('/crates/[crate_id]', { crate_id: dependency.dependentCrateName })}
        class="link"
        onfocusin={() => (focused = true)}
        onfocusout={() => (focused = false)}
        data-test-crate-name
      >
        {dependency.dependentCrateName}
      </a>
      <span class="range">
        depends on
        {dependency.req}
      </span>
    </div>
    <div class="downloads">
      <DownloadArrowIcon class="download-icon" />
      {numberFormat.format(dependency.downloads)}
    </div>
  </div>

  {#await descriptionPromise}
    <div class="description" data-test-description>
      <Placeholder class="description-placeholder" width="70%" height="1em" data-test-placeholder />
    </div>
  {:then description}
    {#if description}
      <div class="description" data-test-description>
        {description}
      </div>
    {/if}
  {:catch}
    <!-- ignore errors and don't display a description -->
  {/await}
</div>

<style>
  .row {
    --hover-bg-color: light-dark(hsl(217, 37%, 98%), hsl(204, 3%, 11%));
    --crate-color: light-dark(var(--grey700), var(--grey600));
    --shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    position: relative;
    font-size: 18px;
    padding: var(--space-s) var(--space-m);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);
    transition: all var(--transition-slow);

    &:hover,
    &.focused {
      background-color: var(--hover-bg-color);
      transition: all var(--transition-instant);
    }

    &.focused {
      box-shadow:
        0 0 0 3px var(--yellow500),
        var(--shadow);
    }
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;

    @media only screen and (max-width: 550px) {
      display: block;
    }
  }

  .left {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .link {
    color: var(--crate-color);
    font-weight: 500;
    margin-right: var(--space-s);
    outline: none;

    &:hover {
      color: var(--crate-color);
    }

    &::after {
      content: '';
      position: absolute;
      left: 0;
      top: 0;
      right: 0;
      bottom: 0;
    }
  }

  .range {
    color: var(--grey600);
    text-transform: uppercase;
    letter-spacing: 0.7px;
    font-size: 13px;
  }

  .downloads {
    display: flex;
    align-items: center;
    color: var(--grey600);
    font-size: 16px;
    font-weight: 500;
    font-variant: tabular-nums;

    @media only screen and (max-width: 550px) {
      margin-top: var(--space-xs);
    }
  }

  .downloads :global(svg.download-icon) {
    width: auto;
    height: 16px;
    flex-shrink: 0;
    margin-right: 7px;
  }

  .description {
    margin-top: var(--space-2xs);
    color: var(--crate-color);
    font-size: 90%;
    line-height: 1.5;

    @media only screen and (max-width: 550px) {
      margin-top: var(--space-xs);
    }
  }
</style>
