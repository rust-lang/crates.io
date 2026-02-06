<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateRow from './CrateRow.svelte';

  const { Story } = defineMeta({
    title: 'CrateRow',
    component: CrateRow,
  });

  type Crate = components['schemas']['Crate'];

  const baseCrate: Crate = {
    id: 'serde',
    name: 'serde',
    default_version: '1.0.215',
    yanked: false,
    description: 'A generic serialization/deserialization framework',
    downloads: 234567890,
    recent_downloads: 12345678,
    updated_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(), // 2 days ago
    created_at: '2014-11-05T00:00:00Z',
    homepage: 'https://serde.rs',
    documentation: 'https://docs.rs/serde',
    repository: 'https://github.com/serde-rs/serde',
    badges: [],
    exact_match: false,
    links: {
      owner_team: '/api/v1/crates/serde/owner_team',
      owner_user: '/api/v1/crates/serde/owner_user',
      reverse_dependencies: '/api/v1/crates/serde/reverse_dependencies',
      version_downloads: '/api/v1/crates/serde/downloads',
      versions: '/api/v1/crates/serde/versions',
    },
    max_version: '1.0.215',
    newest_version: '1.0.215',
    num_versions: 215,
    trustpub_only: false,
  };
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Default</h1>
  <CrateRow crate={baseCrate} />

  <h1>No Description</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      description: null,
    }}
  />

  <h1>Without Links</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'tokio',
      name: 'tokio',
      homepage: null,
      documentation: null,
      repository: null,
    }}
  />

  <h1>Yanked Version</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'yanked-crate',
      name: 'yanked-crate',
      yanked: true,
    }}
  />

  <h1>Long Description</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'long-description',
      name: 'long-description',
      description:
        'This is a very long description that should be truncated because it exceeds the maximum length of 200 characters. It keeps going on and on with more text to demonstrate how the truncation works in the component when dealing with verbose package descriptions.',
    }}
  />

  <h1>Long Crate Name</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'some-very-very-very-very-very-very-very-long-crate-name-that-might-overflow',
      name: 'some-very-very-very-very-very-very-very-long-crate-name-that-might-overflow',
    }}
  />

  <h1>Low Downloads</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'new-crate',
      name: 'new-crate',
      downloads: 42,
      recent_downloads: 15,
      updated_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(), // 1 hour ago
    }}
  />

  <h1>Old Update</h1>
  <CrateRow
    crate={{
      ...baseCrate,
      id: 'old-crate',
      name: 'old-crate',
      updated_at: new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 year ago
    }}
  />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
