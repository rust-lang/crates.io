<script module lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { PlaygroundCrate } from '$lib/utils/playground';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateSidebar from './CrateSidebar.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/CrateSidebar',
    component: CrateSidebar,
    tags: ['autodocs'],
  });

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Owner = components['schemas']['Owner'];

  const playgroundCrates: PlaygroundCrate[] = [{ name: 'serde', version: '1.0.215', id: 'serde' }];

  const playgroundCratesPromise = Promise.resolve(playgroundCrates);

  const baseCrate: Crate = {
    id: 'serde',
    name: 'serde',
    default_version: '1.0.215',
    yanked: false,
    description: 'A generic serialization/deserialization framework',
    downloads: 234567890,
    recent_downloads: 12345678,
    updated_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(),
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

  const baseVersion: Version = {
    id: 12345,
    crate: 'serde',
    num: '1.0.215',
    yanked: false,
    created_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    updated_at: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    downloads: 1234567,
    dl_path: '/api/v1/crates/serde/1.0.215/download',
    readme_path: '/api/v1/crates/serde/1.0.215/readme',
    license: 'MIT OR Apache-2.0',
    edition: '2021',
    rust_version: '1.56',
    checksum: 'e8dfc9d19bdbf6d17e22319da49161d5d0108e4188e8b680aef6299eed22df60',
    crate_size: 123456,
    features: {},
    linecounts: {},
    audit_actions: [],
    links: {
      authors: '/api/v1/crates/serde/1.0.215/authors',
      dependencies: '/api/v1/crates/serde/1.0.215/dependencies',
      version_downloads: '/api/v1/crates/serde/1.0.215/downloads',
    },
  };

  const baseOwners: Owner[] = [
    {
      id: 1,
      kind: 'user',
      name: 'David Tolnay',
      login: 'dtolnay',
      avatar: 'https://avatars.githubusercontent.com/u/1940490?v=4',
    },
    {
      id: 2,
      kind: 'user',
      name: 'Erick Tryzelaar',
      login: 'erickt',
      avatar: 'https://avatars.githubusercontent.com/u/315?v=4',
    },
  ];

  const manyOwners: Owner[] = [
    ...baseOwners,
    { id: 3, kind: 'user', name: 'Alice', login: 'alice', avatar: 'https://avatars.githubusercontent.com/u/3?v=4' },
    { id: 4, kind: 'user', name: 'Bob', login: 'bob', avatar: 'https://avatars.githubusercontent.com/u/4?v=4' },
    { id: 5, kind: 'user', name: 'Charlie', login: 'charlie', avatar: 'https://avatars.githubusercontent.com/u/5?v=4' },
    { id: 6, kind: 'user', name: 'Diana', login: 'diana', avatar: 'https://avatars.githubusercontent.com/u/6?v=4' },
  ];
</script>

<Story name="Default" args={{ crate: baseCrate, version: baseVersion, owners: baseOwners, playgroundCratesPromise }} />

<Story
  name="Yanked"
  args={{ crate: baseCrate, version: { ...baseVersion, yanked: true }, owners: baseOwners, playgroundCratesPromise }}
/>

<Story
  name="Binary Crate"
  args={{
    crate: { ...baseCrate, id: 'cargo-watch', name: 'cargo-watch' },
    version: { ...baseVersion, bin_names: ['cargo-watch'], has_lib: false },
    owners: baseOwners,
    playgroundCratesPromise,
  }}
/>

<Story
  name="Many Owners"
  args={{ crate: baseCrate, version: baseVersion, owners: manyOwners, playgroundCratesPromise }}
/>

<Story
  name="Deduplicated Links"
  args={{
    crate: {
      ...baseCrate,
      homepage: 'https://github.com/serde-rs/serde',
      repository: 'https://github.com/serde-rs/serde',
    },
    version: baseVersion,
    owners: baseOwners,
    playgroundCratesPromise,
  }}
/>

<Story
  name="Minimal"
  args={{
    crate: { ...baseCrate, homepage: null, repository: null },
    version: { ...baseVersion, rust_version: null, edition: null, license: null, crate_size: 0 },
    owners: baseOwners,
    playgroundCratesPromise,
  }}
/>
