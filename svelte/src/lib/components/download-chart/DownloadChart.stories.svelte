<script module lang="ts">
  import type { DownloadChartData, Version, VersionDownload } from './data';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import DownloadChart from './DownloadChart.svelte';

  const { Story } = defineMeta({
    title: 'DownloadChart',
    component: DownloadChart,
    tags: ['autodocs'],
    parameters: {
      chromatic: { diffThreshold: 0.5 },
    },
  });

  function generateData(now: Date): DownloadChartData {
    let versionCount = 5;
    let daysBack = 90;
    let baseDownloads = 10000;

    let versions: Version[] = [];
    let versionDownloads: VersionDownload[] = [];
    let extraDownloads: { date: string; downloads: number }[] = [];

    // Create versions with staggered release dates
    for (let i = 0; i < versionCount; i++) {
      let releaseDay = Math.floor((daysBack / versionCount) * (versionCount - i - 1));
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      let releaseDate = new Date(now);
      releaseDate.setDate(releaseDate.getDate() - releaseDay);

      versions.push(createVersion(i + 1, `1.0.${i}`, releaseDate.toISOString().slice(0, 10)));
    }

    // Generate download data using overlapping sine waves
    for (let day = 0; day < daysBack; day++) {
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      let date = new Date(now);
      date.setDate(date.getDate() - day);
      let dateStr = date.toISOString().slice(0, 10);

      for (let v = 0; v < versions.length; v++) {
        let version = versions[v];
        let versionReleaseDay = Math.floor((daysBack / versionCount) * (versionCount - v - 1));

        // Only generate downloads after version release
        if (day > versionReleaseDay) continue;

        // Each version has a different sine wave pattern
        let frequency = 0.1 + v * 0.05;
        let phase = (v * Math.PI) / 3;
        let amplitude = baseDownloads / (v + 1);

        // Combine multiple sine waves for more interesting patterns
        let wave1 = Math.sin(day * frequency + phase);
        let wave2 = Math.sin(day * frequency * 2.3 + phase) * 0.3;
        let wave3 = Math.sin(day * frequency * 0.5 + phase) * 0.5;

        // Add some decay for older versions
        let decay = Math.exp(-day * 0.02 * (v + 1));

        // Calculate downloads (ensure non-negative)
        let downloads = Math.max(0, Math.round(amplitude * (1 + wave1 + wave2 + wave3) * decay));

        versionDownloads.push({
          version: version.id,
          date: dateStr,
          downloads,
        });
      }

      // Generate extra downloads (for versions outside top N)
      let extraWave = Math.sin(day * 0.08) * 0.5 + Math.sin(day * 0.03) * 0.3;
      let extraAmount = Math.max(0, Math.round(baseDownloads * 0.5 * (1 + extraWave)));
      extraDownloads.push({ date: dateStr, downloads: extraAmount });
    }

    return { versionDownloads, extraDownloads, versions };
  }

  function createVersion(id: number, num: string, createdAt: string): Version {
    return {
      id,
      num,
      audit_actions: [],
      checksum: 'abc123',
      created_at: createdAt,
      crate: 'example-crate',
      crate_size: 50000,
      dl_path: `/api/v1/crates/example-crate/${num}/download`,
      downloads: 100000,
      features: {},
      license: 'MIT',
      linecounts: {},
      links: {
        authors: '',
        dependencies: '',
        version_downloads: '',
      },
      published_by: null,
      readme_path: `/api/v1/crates/example-crate/${num}/readme`,
      updated_at: createdAt,
      yanked: false,
    };
  }

  let now = new Date('2020-12-30T12:34:56Z');
  let defaultData = generateData(now);
</script>

<Story name="Default" args={{ data: defaultData, now, stacked: true }} />

<Story name="Unstacked" args={{ data: defaultData, now, stacked: false }} />

<Story name="No Data" args={{ data: null, now, stacked: true }} parameters={{ chromatic: { disableSnapshot: true } }} />
