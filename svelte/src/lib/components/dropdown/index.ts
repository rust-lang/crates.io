export { default as Root } from './Dropdown.svelte';
export { default as Trigger } from './Trigger.svelte';
export { default as Content } from './Content.svelte';
export { default as Menu } from './Menu.svelte';
export { default as Item } from './MenuItem.svelte';

// Re-export context for advanced usage (e.g., closing dropdown from child component)
export { getDropdown, type DropdownContext } from './context';
