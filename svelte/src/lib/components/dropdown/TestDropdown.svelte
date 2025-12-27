<script lang="ts">
  import * as Dropdown from './index';
</script>

<div data-testid="outside" style="padding: 100px;">
  <!-- Basic dropdown for general tests -->
  <div data-testid="dropdown-with-arrow">
    <Dropdown.Root>
      <Dropdown.Trigger>Open Menu</Dropdown.Trigger>
      <Dropdown.Content>
        <div>Menu content</div>
      </Dropdown.Content>
    </Dropdown.Root>
  </div>

  <!-- Dropdown without arrow -->
  <div data-testid="dropdown-no-arrow">
    <Dropdown.Root>
      <Dropdown.Trigger hideArrow>No Arrow</Dropdown.Trigger>
      <Dropdown.Content>
        <div>Content</div>
      </Dropdown.Content>
    </Dropdown.Root>
  </div>

  <!-- Menu dropdown for menu-specific tests -->
  <div data-testid="dropdown-menu">
    <Dropdown.Root>
      <Dropdown.Trigger>Menu Dropdown</Dropdown.Trigger>
      <Dropdown.Menu>
        <Dropdown.Item><a href="#item1">Item 1</a></Dropdown.Item>
        <Dropdown.Item><a href="#item2">Item 2</a></Dropdown.Item>
        <Dropdown.Item><button type="button">Item 3</button></Dropdown.Item>
      </Dropdown.Menu>
    </Dropdown.Root>
  </div>
</div>

<style>
  [data-testid='outside'] {
    background: #f0f0f0;
  }
</style>
