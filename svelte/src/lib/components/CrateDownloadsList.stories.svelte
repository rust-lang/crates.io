<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateDownloadsList from './CrateDownloadsList.svelte';

  const { Story } = defineMeta({
    title: 'CrateDownloadsList',
    component: CrateDownloadsList,
  });

  type Crate = components['schemas']['Crate'];

  function createCrate(overrides: Partial<Crate> & { id: string; name: string }): Crate {
    return {
      badges: [],
      created_at: '2014-11-05T00:00:00Z',
      default_version: '1.0.0',
      description: 'A sample crate description',
      documentation: 'https://docs.rs/example',
      downloads: 1000000,
      exact_match: false,
      homepage: 'https://example.com',
      links: {
        owner_team: `/api/v1/crates/${overrides.id}/owner_team`,
        owner_user: `/api/v1/crates/${overrides.id}/owner_user`,
        reverse_dependencies: `/api/v1/crates/${overrides.id}/reverse_dependencies`,
        version_downloads: `/api/v1/crates/${overrides.id}/downloads`,
        versions: `/api/v1/crates/${overrides.id}/versions`,
      },
      max_version: '1.0.0',
      newest_version: '1.0.0',
      num_versions: 10,
      recent_downloads: 50000,
      repository: 'https://github.com/example/example',
      trustpub_only: false,
      updated_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(),
      yanked: false,
      ...overrides,
    };
  }

  const sampleCrates: Crate[] = [
    createCrate({
      id: 'serde',
      name: 'serde',
      max_version: '1.0.210',
      downloads: 234567890,
    }),
    createCrate({
      id: 'tokio',
      name: 'tokio',
      max_version: '1.40.0',
      downloads: 123456789,
    }),
    createCrate({
      id: 'reqwest',
      name: 'reqwest',
      max_version: '0.12.8',
      downloads: 98765432,
    }),
  ];
</script>

<Story name="Default" args={{ crates: sampleCrates }} />
