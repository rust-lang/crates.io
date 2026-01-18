import type { TimeUnit } from 'chart.js';
import type { DownloadChartData, Version } from './data';

import {
  _adapters,
  Chart,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  TimeScale,
  Tooltip,
} from 'chart.js';
import { addDays, differenceInDays, endOfDay, format, getTime, startOfDay } from 'date-fns';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

// Register Chart.js components needed for tests
Chart.register(Filler, Legend, LinearScale, LineController, LineElement, PointElement, TimeScale, Tooltip);

// Set up date adapter (same as chartjs.ts)

_adapters._date.override({
  formats: () => ({ day: 'MMM d' }) as Record<TimeUnit | 'datetime', string>,
  parse: (value: unknown) => (value instanceof Date ? value.getTime() : null),
  format: (time: number, fmt: string) => format(time, fmt),
  add: (time: number, amount: number) => getTime(addDays(time, amount)),
  diff: (max: number, min: number) => differenceInDays(max, min),
  startOf: (time: number) => getTime(startOfDay(time)),
  endOf: (time: number) => getTime(endOfDay(time)),
});

// Set up module mock with controllable promise
let resolveChart: (value: typeof Chart) => void;
let rejectChart: (reason: unknown) => void;
let chartPromise: Promise<typeof Chart>;

function resetChartPromise() {
  chartPromise = new Promise((resolve, reject) => {
    resolveChart = resolve;
    rejectChart = reject;
  });
}

vi.mock('./chartjs', () => ({
  loadChart: vi.fn(() => chartPromise),
}));

// Import test wrapper after mock is set up
const { default: TestDownloadChart } = await import('./TestDownloadChart.svelte');

describe('DownloadChart', () => {
  beforeEach(() => {
    resetChartPromise();
  });

  it('happy path', async () => {
    render(TestDownloadChart, { data: exampleData() });
    resolveChart(Chart);

    await expect.element(page.getByCSS('[data-test-download-graph]')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] [data-test-spinner]').query()).toBeNull();
    await expect.element(page.getByCSS('[data-test-download-graph] canvas')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] [data-test-error]').query()).toBeNull();
  });

  it('loading spinner', async () => {
    render(TestDownloadChart, { data: exampleData() });

    await expect.element(page.getByCSS('[data-test-download-graph]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-download-graph] [data-test-spinner]')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] canvas').query()).toBeNull();
    expect(page.getByCSS('[data-test-download-graph] [data-test-error]').query()).toBeNull();

    resolveChart(Chart);

    await expect.element(page.getByCSS('[data-test-download-graph]')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] [data-test-spinner]').query()).toBeNull();
    await expect.element(page.getByCSS('[data-test-download-graph] canvas')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] [data-test-error]').query()).toBeNull();
  });

  it('error behavior', async () => {
    let reloadFn = vi.fn();
    render(TestDownloadChart, { data: exampleData(), onReload: reloadFn });
    rejectChart(new Error('nope'));

    await expect.element(page.getByCSS('[data-test-download-graph]')).toBeVisible();
    expect(page.getByCSS('[data-test-download-graph] [data-test-spinner]').query()).toBeNull();
    expect(page.getByCSS('[data-test-download-graph] canvas').query()).toBeNull();
    await expect.element(page.getByCSS('[data-test-download-graph] [data-test-error]')).toBeVisible();

    await page.getByCSS('[data-test-download-graph] [data-test-reload]').click();
    expect(reloadFn).toHaveBeenCalled();
  });
});

function exampleData(): DownloadChartData {
  let versions: Version[] = [createVersion(52, '1.0.52', '2020-10-01'), createVersion(53, '1.0.53', '2020-10-05')];

  return {
    versionDownloads: [
      { version: 52, date: '2020-12-30', downloads: 201 },
      { version: 53, date: '2020-12-30', downloads: 2228 },
    ],
    extraDownloads: [],
    versions,
  };
}

function createVersion(id: number, num: string, createdAt: string): Version {
  return {
    id,
    num,
    audit_actions: [],
    checksum: 'abc123',
    created_at: createdAt,
    crate: 'test-crate',
    crate_size: 1000,
    dl_path: `/api/v1/crates/test-crate/${num}/download`,
    downloads: 0,
    features: {},
    license: 'MIT',
    linecounts: {},
    links: {
      authors: '',
      dependencies: '',
      version_downloads: '',
    },
    published_by: null,
    readme_path: `/api/v1/crates/test-crate/${num}/readme`,
    updated_at: createdAt,
    yanked: false,
  };
}
