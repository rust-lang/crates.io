<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateHeader from './CrateHeader.svelte';

  const { Story } = defineMeta({
    title: 'CrateHeader',
    component: CrateHeader,
    tags: ['autodocs'],
  });

  type Crate = components['schemas']['Crate'];
  type Version = components['schemas']['Version'];
  type Keyword = components['schemas']['Keyword'];

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
    created_at: '2024-01-15T10:00:00Z',
    updated_at: '2024-01-15T10:00:00Z',
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

  const sampleKeywords: Keyword[] = [
    { id: 'serialization', keyword: 'serialization', crates_cnt: 1234, created_at: '2015-01-01T00:00:00Z' },
    { id: 'encoding', keyword: 'encoding', crates_cnt: 567, created_at: '2015-01-01T00:00:00Z' },
    { id: 'json', keyword: 'json', crates_cnt: 890, created_at: '2015-01-01T00:00:00Z' },
  ];
</script>

<Story name="Default" args={{ crate: baseCrate }} />

<Story name="With Version" args={{ crate: baseCrate, version: baseVersion, versionNum: '1.0.215' }} />

<Story
  name="With Keywords"
  args={{
    crate: baseCrate,
    version: baseVersion,
    versionNum: '1.0.215',
    keywords: sampleKeywords,
  }}
/>

<Story
  name="Yanked Version"
  args={{
    crate: baseCrate,
    version: {
      ...baseVersion,
      yanked: true,
    },
    versionNum: '1.0.215',
    keywords: sampleKeywords,
  }}
/>

<Story
  name="No Description"
  args={{
    crate: {
      ...baseCrate,
      description: null,
    },
    version: baseVersion,
    versionNum: '1.0.215',
  }}
/>

<Story
  name="Single Version"
  args={{
    crate: {
      ...baseCrate,
      num_versions: 1,
    },
    version: baseVersion,
    versionNum: '1.0.215',
  }}
/>

<Story
  name="Long Crate Name"
  args={{
    crate: {
      ...baseCrate,
      id: 'some-very-very-very-very-very-very-long-crate-name',
      name: 'some-very-very-very-very-very-very-long-crate-name',
    },
    version: baseVersion,
    versionNum: '1.0.215',
    keywords: sampleKeywords,
  }}
/>

<Story
  name="Long Description"
  args={{
    crate: {
      ...baseCrate,
      description:
        'This is a very long description that explains what this crate does in great detail. It includes information about the serialization framework, the supported formats, and the various features that make it useful for Rust developers who need to serialize and deserialize data structures.',
    },
    version: baseVersion,
    versionNum: '1.0.215',
  }}
/>
