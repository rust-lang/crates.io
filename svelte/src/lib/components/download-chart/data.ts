import type { components } from '@crates-io/api-client';
import type { ChartDataset } from 'chart.js';

import { addMinutes, subDays as brokenSubDays, subMinutes } from 'date-fns';
import semverSort from 'semver/functions/sort';

// Re-export API types for convenience
export type Version = components['schemas']['Version'];
export type VersionDownload = components['schemas']['VersionDownload'];

export interface DownloadChartData {
  versionDownloads: VersionDownload[];
  extraDownloads: { date: string; downloads: number }[];
  versions: Version[];
  crateCreatedAt?: string;
}

export type DownloadChartDataset = ChartDataset<'line', { x: Date; y: number }[]>;

// Colors by http://colorbrewer2.org/#type=diverging&scheme=RdBu&n=10
const COLORS = ['#67001f', '#b2182b', '#d6604d', '#f4a582', '#92c5de', '#4393c3', '#2166ac', '#053061'];
const BG_COLORS = ['#d3b5bc', '#eabdc0', '#f3d0ca', '#fce4d9', '#deedf5', '#c9deed', '#2166ac', '#053061'];

const ONE_DAY = 24 * 60 * 60 * 1000;

export function toChartData(data: DownloadChartData | null, now: Date): { datasets: DownloadChartDataset[] } {
  if (!data) {
    return { datasets: [] };
  }

  let extra = data.extraDownloads;
  let crateCreatedAt = data.crateCreatedAt;

  let dates: Record<string, { date: Date; cnt: Record<string, number> }> = {};
  let versions = new Map<string, Version>();
  let versionsById = new Map(data.versions.map(v => [v.id, v]));

  for (let i = 0; i < 90; i++) {
    let date = subDays(now, i);
    dates[date.toISOString().slice(0, 10)] = { date, cnt: {} };
  }

  data.versionDownloads.forEach(d => {
    let version = versionsById.get(d.version);
    if (!version) return;

    let version_num = version.num;

    versions.set(version_num, version);

    let key = d.date;
    if (dates[key]) {
      let prev = dates[key].cnt[version_num] || 0;
      dates[key].cnt[version_num] = prev + d.downloads;
    }
  });

  extra.forEach(d => {
    let key = d.date;
    if (dates[key]) {
      let prev = dates[key].cnt['Other'] || 0;
      dates[key].cnt['Other'] = prev + d.downloads;
    }
  });

  let versionsList = [...versions.keys()];
  try {
    semverSort(versionsList, { loose: true });
  } catch {
    // Catches exceptions thrown when a version number is invalid
    // see issue #3295
  }

  if (extra.length !== 0) {
    versionsList.unshift('Other');
  }

  let rows = Object.keys(dates).map(
    date => [dates[date].date, ...versionsList.map(version => dates[date].cnt[version] || 0)] as [Date, ...number[]],
  );

  let datasets = versionsList
    .map((label, index) => {
      let data = rows.map(row => ({ x: row[0], y: row[index + 1] as number }));

      // if we find a corresponding version
      let version = versions.get(label);
      if (version?.created_at) {
        // only show downloads from the day before the release until today
        let threshold = midnightForDate(new Date(version.created_at)) - ONE_DAY;
        data = data.filter(it => midnightForDate(it.x) >= threshold);
      }

      // if we find a corresponding crate
      if (crateCreatedAt) {
        // only show downloads from the day before the first release until today
        let threshold = midnightForDate(new Date(crateCreatedAt)) - ONE_DAY;
        data = data.filter(it => midnightForDate(it.x) >= threshold);
      }

      return { data, label };
    })
    .reverse()
    .map(({ label, data }, index): DownloadChartDataset => {
      return {
        backgroundColor: BG_COLORS[index],
        borderColor: COLORS[index],
        borderWidth: 2,
        cubicInterpolationMode: 'monotone',
        data: data,
        // TODO figure out why this doesn't work. is it a chart.js bug?
        fill: 'origin',
        label: label,
        pointHoverBorderWidth: 2,
        pointHoverRadius: 5,
      };
    });

  return { datasets };
}

function midnightForDate(date: Date): number {
  return Date.parse(date.toISOString().slice(0, 10));
}

// This works around a bug in date-fn's subDays() function when crossing a DST
// transition: https://github.com/date-fns/date-fns/issues/571
//
// The specific implementation is based on @bertho-zero's here:
// https://github.com/date-fns/date-fns/issues/571#issuecomment-602496322
function subDays(date: Date, amount: number): Date {
  let originalTZO = date.getTimezoneOffset();
  let endDate = brokenSubDays(date, amount);
  let endTZO = endDate.getTimezoneOffset();

  let dstDiff = originalTZO - endTZO;

  return dstDiff >= 0 ? addMinutes(endDate, dstDiff) : subMinutes(endDate, Math.abs(dstDiff));
}
