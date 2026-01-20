<script lang="ts">
  import type { Chart } from 'chart.js';
  import type { DownloadChartData } from './data';

  import { untrack } from 'svelte';

  import { getColorScheme } from '$lib/color-scheme.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import { loadChart } from './chartjs';
  import { toChartData } from './data';

  interface Props {
    data: DownloadChartData | null;
    now?: Date;
    /** Whether to display the chart as a stacked area chart */
    stacked?: boolean;
    onReload?: () => void;
  }

  let { data, now = new Date(), stacked = true, onReload = () => location.reload() }: Props = $props();

  let colorScheme = getColorScheme();

  let chart: Chart<'line', { x: Date; y: number }[]> | null = $state(null);
  let canvasRef: HTMLCanvasElement | null = $state(null);
  let ChartConstructor: typeof Chart<'line', { x: Date; y: number }[]> | null = $state(null);

  // Load Chart.js and capture the constructor when ready
  let chartPromise = loadChart().then(Chart => (ChartConstructor = Chart));

  let chartData = $derived(toChartData(data, now));
  let fontColor = $derived(colorScheme.isDark ? '#ADBABD' : '#666');
  let borderColor = $derived(colorScheme.isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)');

  // Create chart when canvas and Chart.js are ready
  $effect(() => {
    if (!canvasRef || !ChartConstructor) return;

    // Read initial values without creating dependencies
    let initialData = untrack(() => chartData);
    let initialFontColor = untrack(() => fontColor);
    let initialBorderColor = untrack(() => borderColor);
    let initialStacked = untrack(() => stacked);

    chart = new ChartConstructor(canvasRef, {
      type: 'line',
      data: initialData,
      options: {
        color: initialFontColor,
        maintainAspectRatio: false,
        layout: { padding: 10 },
        scales: {
          x: {
            type: 'time',
            time: { tooltipFormat: 'MMM d', unit: 'day' },
            ticks: { maxTicksLimit: 13, color: initialFontColor },
            grid: { color: initialBorderColor },
          },
          y: {
            beginAtZero: true,
            stacked: initialStacked,
            ticks: { precision: 0, color: initialFontColor },
            grid: { color: initialBorderColor },
          },
        },
        interaction: { mode: 'index', intersect: false },
        plugins: { tooltip: { position: 'nearest' } },
      },
    });

    // Cleanup chart on component destroy
    return () => {
      chart?.destroy();
      chart = null;
    };
  });

  // Update chart data
  $effect(() => {
    let untrackedChart = untrack(() => chart);
    if (untrackedChart) {
      untrackedChart.data = chartData;
      untrackedChart.update();
    }
  });

  // Update colors on theme change
  $effect(() => {
    let untrackedChart = untrack(() => chart);
    if (untrackedChart) {
      untrackedChart.options.color = fontColor;
      untrackedChart.options.scales!.x!.ticks!.color = fontColor;
      untrackedChart.options.scales!.x!.grid!.color = borderColor;
      untrackedChart.options.scales!.y!.ticks!.color = fontColor;
      untrackedChart.options.scales!.y!.grid!.color = borderColor;
      untrackedChart.update();
    }
  });

  // Update stacked mode
  $effect(() => {
    if (chart) {
      for (let dataset of chart.data.datasets) {
        dataset.fill = stacked ? 'origin' : false;
      }
      chart.options.scales!.y!.stacked = stacked;
      chart.update();
    }
  });
</script>

<div data-test-download-graph class="wrapper">
  {#await chartPromise}
    <LoadingSpinner class="spinner" data-test-spinner />
  {:then}
    <canvas bind:this={canvasRef}></canvas>
  {:catch}
    <div class="error" data-test-error>
      <p>Sorry, there was a problem loading the graphing code.</p>
      <button type="button" data-test-reload onclick={onReload}>Try again</button>
    </div>
  {/await}
</div>

<style>
  .wrapper {
    display: grid;
    place-items: center;
    border: solid 1px var(--gray-border);
    border-radius: var(--space-3xs);
    min-height: 400px;
  }

  .wrapper :global(.spinner) {
    transform: scale(3);
  }

  .error {
    text-align: center;
  }
</style>
