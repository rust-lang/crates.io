<script lang="ts">
  import { SearchFormContext, setSearchFormContext } from '#lib/search-form.svelte.ts';
  import SearchForm from './SearchForm.svelte';

  setSearchFormContext(new SearchFormContext());
</script>

<SearchForm />
