<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateRow from './CrateRow.svelte';

  const { Story } = defineMeta({
    title: 'CrateRow',
    component: CrateRow,
    tags: ['autodocs'],
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

<Story name="Default" args={{ crate: baseCrate }} />

<Story
  name="No Description"
  args={{
    crate: {
      ...baseCrate,
      description: null,
    },
  }}
/>

<Story
  name="Without Links"
  args={{
    crate: {
      ...baseCrate,
      id: 'tokio',
      name: 'tokio',
      homepage: null,
      documentation: null,
      repository: null,
    },
  }}
/>

<Story
  name="Yanked Version"
  args={{
    crate: {
      ...baseCrate,
      id: 'yanked-crate',
      name: 'yanked-crate',
      yanked: true,
    },
  }}
/>

<Story
  name="Long Description"
  args={{
    crate: {
      ...baseCrate,
      id: 'long-description',
      name: 'long-description',
      description:
        'This is a very long description that should be truncated because it exceeds the maximum length of 200 characters. It keeps going on and on with more text to demonstrate how the truncation works in the component when dealing with verbose package descriptions.',
    },
  }}
/>

<Story
  name="Long Crate Name"
  args={{
    crate: {
      ...baseCrate,
      id: 'some-very-very-very-very-very-very-very-long-crate-name-that-might-overflow',
      name: 'some-very-very-very-very-very-very-very-long-crate-name-that-might-overflow',
    },
  }}
/>

<Story
  name="Low Downloads"
  args={{
    crate: {
      ...baseCrate,
      id: 'new-crate',
      name: 'new-crate',
      downloads: 42,
      recent_downloads: 15,
      updated_at: new Date(Date.now() - 60 * 60 * 1000).toISOString(), // 1 hour ago
    },
  }}
/>

<Story
  name="Old Update"
  args={{
    crate: {
      ...baseCrate,
      id: 'old-crate',
      name: 'old-crate',
      updated_at: new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString(), // 1 year ago
    },
  }}
/>
