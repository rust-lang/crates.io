import { createContext } from 'svelte';

export interface DropdownContext {
  readonly isExpanded: boolean;
  readonly triggerId: string;
  readonly contentId: string;
  toggle: () => void;
  close: () => void;
}

export const [getDropdown, setDropdown] = createContext<DropdownContext>();
