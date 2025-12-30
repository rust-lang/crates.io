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

<Story name="Text Only" asChild>
  <div style="display: flex; justify-content: center; padding-top: 20px;">
    <button type="button">
      Hover me
      <Tooltip text="This is a simple text tooltip" />
    </button>
  </div>
</Story>

<Story name="Block Content" asChild>
  <div style="display: flex; justify-content: center; padding-top: 50px">
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
</Story>

<Story name="Side: Top (Default)" asChild>
  <div style="display: flex; justify-content: center; padding: 20px;">
    <button type="button">
      Top tooltip
      <Tooltip text="Positioned above the element" side="top" />
    </button>
  </div>
</Story>

<Story name="Side: Bottom" asChild>
  <div style="display: flex; justify-content: center; padding: 20px;">
    <button type="button">
      Bottom tooltip
      <Tooltip text="Positioned below the element" side="bottom" />
    </button>
  </div>
</Story>

<Story name="Side: Left" asChild>
  <div style="display: flex; justify-content: center; padding: 20px;">
    <button type="button">
      Left tooltip
      <Tooltip text="Positioned to the left" side="left" />
    </button>
  </div>
</Story>

<Story name="Side: Right" asChild>
  <div style="display: flex; justify-content: center; padding: 20px;">
    <button type="button">
      Right tooltip
      <Tooltip text="Positioned to the right" side="right" />
    </button>
  </div>
</Story>

<Story name="Multiple Tooltips" asChild>
  <div style="display: flex; gap: 20px; justify-content: center; padding-top: 20px;">
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
</Story>

<Story name="On Link" asChild>
  <div style="display: flex; justify-content: center; padding-top: 20px;">
    <a href="#example">
      Hover this link
      <Tooltip text="Links can have tooltips too" />
    </a>
  </div>
</Story>

<Story name="On Inline Element" asChild>
  <div style="display: flex; justify-content: center; padding-top: 50px;">
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
