<script lang="ts">
  import type { DownloadChartData } from './data';

  import { setColorScheme } from '$lib/color-scheme.svelte';
  import DownloadChart from './DownloadChart.svelte';

  interface Props {
    data: DownloadChartData | null;
    stacked?: boolean;
    onReload?: () => void;
  }

  let { data, stacked = true, onReload }: Props = $props();

  // Provide a mock color scheme context
  setColorScheme({
    scheme: 'system',
    resolvedScheme: 'light',
    isDark: false,
    setScheme: () => {},
  });
</script>

<DownloadChart {data} {stacked} {onReload} />
