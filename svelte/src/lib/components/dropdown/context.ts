import { createContext } from 'svelte';

export interface DropdownContext {
  isExpanded: boolean;
  trigger: HTMLButtonElement | undefined;
  readonly triggerId: string;
  readonly contentId: string;
  toggle: () => void;
  close: () => void;
}

export const [getDropdown, setDropdown] = createContext<DropdownContext>();
