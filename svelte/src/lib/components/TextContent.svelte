<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    boxed?: boolean;
    children: Snippet;
  }

  let { boxed = false, children, class: className, ...others }: Props = $props();
</script>

<div class={['wrapper', className, { boxed }]} {...others}>
  {@render children()}
</div>

<style>
  .boxed {
    padding: var(--space-m);
    background-color: light-dark(white, #141413);
    margin-bottom: var(--space-s);
    border-radius: 5px;
  }

  .wrapper {
    --icon-note: icon('i-octicon:info-16');
    --icon-tip: icon('i-octicon:light-bulb-16');
    --icon-important: icon('i-octicon:report-16');
    --icon-warning: icon('i-octicon:alert-16');
    --icon-caution: icon('i-octicon:stop-16');

    line-height: 1.5;
    overflow-wrap: break-word;

    > :global(:first-child) {
      margin-top: 0;
    }

    > :global(:last-child) {
      margin-bottom: 0;
    }

    :global(img) {
      max-width: 100%;
    }

    :global(pre) {
      display: block;
      overflow-x: auto;
      padding: var(--space-xs);
      background-color: light-dark(#f6f8fa, #161b22);
      font-size: 85%;
      border-radius: var(--space-3xs);
    }

    :global(p),
    :global(li) {
      :global(code) {
        background-color: light-dark(#f6f8fa, #383836);
        border-radius: var(--space-3xs);
        font-size: 85%;
        margin: 0;
        padding: var(--space-4xs) var(--space-3xs);
      }
    }

    :global(code) {
      font-family: var(--font-monospace);
      tab-size: 4;
    }

    :global(kbd) {
      font-family: var(--font-monospace);
      font-size: 11px;

      padding: 2px 5px 3px 5px;

      border-radius: 7px;

      position: relative;
      bottom: 2px;

      border: 1px solid var(--grey700);
      box-shadow: inset 0 -2px 0 var(--grey600);
    }

    :global(table) {
      border-collapse: collapse;
      display: block;
      overflow-x: auto;

      :global(th),
      :global(td) {
        border: 1px solid #dfe2e5;
        padding: var(--space-2xs) var(--space-s);
      }
    }

    :global(section.footnotes) {
      color: var(--main-color-light);
      font-size: 80%;
      border-top: 1px solid var(--gray-border);

      :global(a) {
        color: var(--main-color-light);

        &:hover {
          color: var(--main-color);
        }
      }
    }

    /* alerts */
    :global(.markdown-alert) {
      --fg-color-note: #4494f8;
      --fg-color-tip: #3fb950;
      --fg-color-important: #ab7df8;
      --fg-color-warning: #d29922;
      --fg-color-caution: #f85149;

      padding: 0.5rem 1rem;
      margin-bottom: 1rem;
      color: inherit;
      border-left: 0.25em solid var(--gray-border);

      & > :global(:first-child) {
        margin-top: 0;
      }

      & > :global(:last-child) {
        margin-bottom: 0;
      }

      :global(.markdown-alert-title) {
        display: flex;
        font-weight: 500;
        align-items: center;
        line-height: 1;
      }

      & > :global(.markdown-alert-title)::before {
        content: '';
        margin-right: 0.5rem;
        background-color: var(--gray-border);
        width: 1em;
        height: 1em;
      }

      &:global(.markdown-alert-note) {
        border-left-color: var(--fg-color-note);

        & > :global(.markdown-alert-title) {
          color: var(--fg-color-note);

          &:before {
            mask: var(--icon-note);
            background-color: var(--fg-color-note);
          }
        }
      }

      &:global(.markdown-alert-tip) {
        border-left-color: var(--fg-color-tip);

        & > :global(.markdown-alert-title) {
          color: var(--fg-color-tip);

          &:before {
            mask: var(--icon-tip);
            background-color: var(--fg-color-tip);
          }
        }
      }

      &:global(.markdown-alert-important) {
        border-left-color: var(--fg-color-important);

        & > :global(.markdown-alert-title) {
          color: var(--fg-color-important);

          &:before {
            mask: var(--icon-important);
            background-color: var(--fg-color-important);
          }
        }
      }

      &:global(.markdown-alert-warning) {
        border-left-color: var(--fg-color-warning);

        & > :global(.markdown-alert-title) {
          color: var(--fg-color-warning);

          &:before {
            mask: var(--icon-warning);
            background-color: var(--fg-color-warning);
          }
        }
      }

      &:global(.markdown-alert-caution) {
        border-left-color: var(--fg-color-caution);

        & > :global(.markdown-alert-title) {
          color: var(--fg-color-caution);

          &:before {
            mask: var(--icon-caution);
            background-color: var(--fg-color-caution);
          }
        }
      }
    }
  }
</style>
