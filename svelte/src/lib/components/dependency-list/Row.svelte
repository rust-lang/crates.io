<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import CheckboxEmptyIcon from '$lib/assets/checkbox-empty.svg?component';
  import CheckboxIcon from '$lib/assets/checkbox.svg?component';
  import Placeholder from '$lib/components/Placeholder.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';

  type Dependency = components['schemas']['EncodableDependency'];

  interface Props {
    dependency: Dependency;
    descriptionPromise: Promise<string | null> | undefined;
  }

  let { dependency, descriptionPromise }: Props = $props();

  let focused = $state(false);

  let formattedReq = $derived(dependency.req === '*' ? '' : dependency.req);

  let featuresDescription = $derived.by(() => {
    let { default_features: defaultFeatures, features } = dependency;
    let numFeatures = features.length;

    if (numFeatures !== 0) {
      return defaultFeatures
        ? `${numFeatures} extra feature${numFeatures > 1 ? 's' : ''}`
        : `only ${numFeatures} feature${numFeatures > 1 ? 's' : ''}`;
    } else if (!defaultFeatures) {
      return 'no default features';
    }
  });
</script>

<div data-test-dependency={dependency.crate_id} class="row" class:optional={dependency.optional} class:focused>
  <span class="range-lg" data-test-range>
    {formattedReq}
  </span>

  <div class="right">
    <div class="name-and-metadata">
      <span class="range-sm">
        {formattedReq}
      </span>

      <a
        href={resolve('/crates/[crate_id]/range/[range]', { crate_id: dependency.crate_id, range: dependency.req })}
        class="link"
        onfocusin={() => (focused = true)}
        onfocusout={() => (focused = false)}
        data-test-crate-name
      >
        {dependency.crate_id}
      </a>

      {#if dependency.optional}
        <span class="optional-label" data-test-optional>optional</span>
      {/if}

      {#if featuresDescription}
        <span class="features-label" data-test-features>
          {featuresDescription}

          <Tooltip>
            <ul class="feature-list">
              <li>
                {#if dependency.default_features}
                  <CheckboxIcon />
                {:else}
                  <CheckboxEmptyIcon />
                {/if}
                default features
              </li>
              {#each dependency.features as feature (feature)}
                <li>
                  <CheckboxIcon />
                  {feature}
                </li>
              {/each}
            </ul>
          </Tooltip>
        </span>
      {/if}
    </div>

    {#await descriptionPromise}
      <div class="description">
        <Placeholder
          class="description-placeholder"
          width="70%"
          height="1em"
          opacity={dependency.optional ? 0.15 : 0.35}
        />
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
</div>

<style>
  .row {
    --bg-color: var(--grey200);
    --hover-bg-color: light-dark(hsl(217, 37%, 98%), hsl(204, 3%, 11%));
    --range-color: light-dark(var(--grey900), #d1cfc7);
    --crate-color: light-dark(var(--grey700), #d1cfc7);
    --shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    display: flex;
    align-items: center;
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

    &.optional {
      --range-color: light-dark(var(--grey600), var(--grey600));
      --crate-color: light-dark(var(--grey600), var(--grey600));
    }

    .features-label {
      position: relative;
      z-index: 1;
      cursor: help;
    }

    @media only screen and (max-width: 550px) {
      display: block;
    }
  }

  .range-lg,
  .range-sm {
    margin-right: var(--space-s);
    min-width: 100px;
    color: var(--range-color);
    font-variant: tabular-nums;
  }

  .range-lg {
    @media only screen and (max-width: 550px) {
      display: none;
    }
  }

  .range-sm {
    @media only screen and (min-width: 551px) {
      display: none;
    }
  }

  .right {
    flex-grow: 1;
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

  .optional-label,
  .features-label {
    color: var(--grey600);
    text-transform: uppercase;
    letter-spacing: 0.7px;
    font-size: 13px;
    margin-right: var(--space-s);

    @media only screen and (max-width: 550px) {
      display: block;
      margin-top: var(--space-xs);
    }
  }

  .feature-list {
    padding: 0;
    margin: var(--space-2xs) var(--space-3xs);
    list-style: none;

    :global(svg) {
      height: 1em;
      width: auto;
      margin-right: 2px;
      margin-bottom: -0.1em;
    }
  }

  .description {
    margin-top: var(--space-xs);
    color: var(--crate-color);
    font-size: 90%;
    line-height: 1.5;
  }
</style>
