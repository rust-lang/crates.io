<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Tooltip from './Tooltip.svelte';

  const { Story } = defineMeta({
    title: 'Tooltip',
    component: Tooltip,
    tags: ['autodocs'],
    argTypes: {
      text: {
        control: 'text',
      },
      side: {
        control: 'inline-radio',
        options: ['top', 'bottom', 'left', 'right'],
      },
      children: {
        table: {
          disable: true,
        },
      },
    },
  });
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <h1>Text Only</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Hover me
      <Tooltip text="This is a simple text tooltip" />
    </button>
  </div>

  <h1>Block Content</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Hover for details
      <Tooltip>
        <strong>Package URL:</strong><br />
        pkg:cargo/serde@1.0.0
        <br />
        <small>(click to copy)</small>
      </Tooltip>
    </button>
  </div>

  <h1>Side: Top (Default)</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Top tooltip
      <Tooltip text="Positioned above the element" side="top" />
    </button>
  </div>

  <h1>Side: Bottom</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Bottom tooltip
      <Tooltip text="Positioned below the element" side="bottom" />
    </button>
  </div>

  <h1>Side: Left</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Left tooltip
      <Tooltip text="Positioned to the left" side="left" />
    </button>
  </div>

  <h1>Side: Right</h1>
  <div style="display: flex; justify-content: center;">
    <button type="button">
      Right tooltip
      <Tooltip text="Positioned to the right" side="right" />
    </button>
  </div>

  <h1>Multiple Tooltips</h1>
  <div style="display: flex; gap: 20px; justify-content: center;">
    <button type="button">
      First
      <Tooltip text="Tooltip for the first button" />
    </button>
    <button type="button">
      Second
      <Tooltip text="Tooltip for the second button" />
    </button>
    <button type="button">
      Third
      <Tooltip text="Tooltip for the third button" />
    </button>
  </div>

  <h1>On Link</h1>
  <div style="display: flex; justify-content: center;">
    <a href="#example">
      Hover this link
      <Tooltip text="Links can have tooltips too" />
    </a>
  </div>

  <h1>On Inline Element</h1>
  <div style="display: flex; justify-content: center;">
    <p>
      Some text with a
      <span style="text-decoration: underline; cursor: help;">
        tooltip term
        <Tooltip>
          <strong>Definition:</strong><br />
          A tooltip is a small popup that appears on hover.
        </Tooltip>
      </span>in the middle.
    </p>
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
    text-align: center;
  }
</style>
