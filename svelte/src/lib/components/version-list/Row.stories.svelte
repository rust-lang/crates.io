<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Row from './Row.svelte';

  type Version = components['schemas']['Version'];

  const { Story } = defineMeta({
    title: 'version-list/Row',
    component: Row,
  });

  interface TrustpubData {
    provider: string;
    repository?: string;
    run_id?: string;
    project_path?: string;
    job_id?: string;
  }

  // TODO this is supposed to just be `Partial<Version>` but TS is currently unhappy
  // with the type signature of some of the fields derived from the OpenAPI spec.
  interface VersionOverrides extends Partial<Omit<Version, 'features' | 'trustpub_data' | 'linecounts'>> {
    features?: Record<string, string[]>;
    trustpub_data?: TrustpubData | null;
  }

  function createVersion(overrides: VersionOverrides = {}): Version {
    let { features: featuresOverride, trustpub_data: trustpubOverride, ...rest } = overrides;
    let features = (featuresOverride ?? {
      default: ['std'],
      std: [],
      derive: ['serde_derive'],
      alloc: [],
      unstable: [],
    }) as Version['features'];

    return {
      id: 1,
      crate: 'serde',
      num: '1.0.200',
      created_at: new Date(Date.now() - 2 * 24 * 60 * 60 * 1000).toISOString(),
      downloads: 1234567,
      crate_size: 85000,
      license: 'MIT OR Apache-2.0',
      rust_version: '1.56.0',
      edition: '2021',
      yanked: false,
      checksum: 'abc123',
      dl_path: '/api/v1/crates/serde/1.0.200/download',
      readme_path: '/api/v1/crates/serde/1.0.200/readme',
      updated_at: new Date().toISOString(),
      audit_actions: [],
      links: {
        authors: '/api/v1/crates/serde/1.0.200/authors',
        dependencies: '/api/v1/crates/serde/1.0.200/dependencies',
        version_downloads: '/api/v1/crates/serde/1.0.200/downloads',
      },
      linecounts: {},
      published_by: {
        id: 1,
        login: 'dtolnay',
        name: 'David Tolnay',
        avatar: 'https://avatars.githubusercontent.com/u/1940490?v=4',
        url: 'https://github.com/dtolnay',
      },
      ...rest,
      features,
      trustpub_data: trustpubOverride as Version['trustpub_data'],
    };
  }
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Default" asChild>
  <h1>Latest Version</h1>
  <Row version={createVersion()} crateName="serde" isHighestOfReleaseTrack={true} />

  <h1>Regular Version</h1>
  <Row version={createVersion()} crateName="serde" isHighestOfReleaseTrack={false} />

  <h1>Prerelease</h1>
  <Row version={createVersion({ num: '2.0.0-alpha.1' })} crateName="serde" />

  <h1>Yanked</h1>
  <Row version={createVersion({ yanked: true })} crateName="serde" />

  <h1>New Version</h1>
  <Row
    version={createVersion({ created_at: new Date().toISOString() })}
    crateName="serde"
    isHighestOfReleaseTrack={true}
  />

  <h1>With Trusted Publisher (GitHub)</h1>
  <Row
    version={createVersion({
      published_by: null,
      trustpub_data: { provider: 'github', repository: 'serde-rs/serde', run_id: '12345678' },
    })}
    crateName="serde"
    isHighestOfReleaseTrack={true}
  />

  <h1>With Trusted Publisher (GitLab)</h1>
  <Row
    version={createVersion({
      published_by: null,
      trustpub_data: { provider: 'gitlab', project_path: 'serde-rs/serde', job_id: '87654321' },
    })}
    crateName="serde"
    isHighestOfReleaseTrack={true}
  />

  <h1>Minimal Version</h1>
  <Row
    version={createVersion({
      crate_size: 0,
      license: undefined,
      rust_version: undefined,
      edition: undefined,
      features: {},
      published_by: undefined,
    })}
    crateName="minimal-crate"
  />

  <h1>With Edition Only</h1>
  <Row version={createVersion({ rust_version: null, edition: '2021' })} crateName="serde" />

  <h1>Single Feature</h1>
  <Row version={createVersion({ features: { default: [], derive: [] } })} crateName="serde" />

  <h1>Many Features</h1>
  <Row
    version={createVersion({
      features: {
        default: ['std', 'derive'],
        std: [],
        derive: ['serde_derive'],
        alloc: [],
        unstable: [],
        rc: [],
        serde_derive: [],
        feature7: [],
        feature8: [],
        feature9: [],
        feature10: [],
        feature11: [],
        feature12: [],
        feature13: [],
        feature14: [],
        feature15: [],
        feature16: [],
        feature17: [],
      },
    })}
    crateName="serde"
  />

  <h1>Invalid Semver</h1>
  <Row
    version={createVersion({ num: '18446744073709551615.18446744073709551615.18446744073709551615' })}
    crateName="foo"
  />

  <h1>Zero Major Version</h1>
  <Row version={createVersion({ num: '0.4.0' })} crateName="new-crate" isHighestOfReleaseTrack={true} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    color: var(--grey500);
    margin: 1rem 0 0.25rem;
  }
</style>
