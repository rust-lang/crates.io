import { createContext } from 'svelte';

export interface TooltipContext {
  readonly containerId: string;
}

export const [getTooltipContext, setTooltipContext] = createContext<TooltipContext>();
