import { createContext } from 'svelte';

export class SearchFormContext {
  value = $state('');
}

export const [getSearchFormContext, setSearchFormContext] = createContext<SearchFormContext>();
