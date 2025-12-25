<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CrateLists from './CrateLists.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/CrateLists',
    component: CrateLists,
    tags: ['autodocs'],
  });

  function createMockCrate(name: string, version: string) {
    return {
      id: name,
      name,
      newest_version: version,
      max_version: version,
      downloads: 1000000,
      created_at: '2020-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      badges: [],
      exact_match: false,
      links: {
        reverse_dependencies: `/api/v1/crates/${name}/reverse_dependencies`,
        version_downloads: `/api/v1/crates/${name}/downloads`,
      },
      num_versions: 10,
      trustpub_only: false,
      yanked: false,
    };
  }

  function createMockKeyword(id: string, crates_cnt: number) {
    return {
      id,
      keyword: id,
      crates_cnt,
      created_at: '2020-01-01T00:00:00Z',
    };
  }

  function createMockCategory(id: string, name: string, crates_cnt: number) {
    return {
      id,
      slug: id,
      category: name,
      description: `${name} related crates`,
      crates_cnt,
      created_at: '2020-01-01T00:00:00Z',
    };
  }

  const mockSummary = {
    num_downloads: 123456789,
    num_crates: 150000,
    new_crates: [
      createMockCrate('new-crate-1', '0.1.0'),
      createMockCrate('new-crate-2', '0.2.0'),
      createMockCrate('new-crate-3', '0.1.0'),
    ],
    most_downloaded: [
      createMockCrate('serde', '1.0.200'),
      createMockCrate('tokio', '1.37.0'),
      createMockCrate('rand', '0.8.5'),
    ],
    most_recently_downloaded: [
      createMockCrate('syn', '2.0.60'),
      createMockCrate('quote', '1.0.36'),
      createMockCrate('proc-macro2', '1.0.81'),
    ],
    just_updated: [
      createMockCrate('updated-crate-1', '2.0.0'),
      createMockCrate('updated-crate-2', '1.5.0'),
      createMockCrate('updated-crate-3', '3.0.0-beta.1'),
    ],
    popular_keywords: [
      createMockKeyword('async', 5000),
      createMockKeyword('http', 3500),
      createMockKeyword('cli', 2800),
    ],
    popular_categories: [
      createMockCategory('command-line-utilities', 'Command-line utilities', 4500),
      createMockCategory('web-programming', 'Web programming', 3800),
      createMockCategory('development-tools', 'Development tools', 3200),
    ],
  };
</script>

<Story name="Loaded" args={{ summary: mockSummary }} />

<Story name="Loading" />
