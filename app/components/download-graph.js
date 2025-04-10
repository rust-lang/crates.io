import { action } from '@ember/object';
import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';
import Component from '@glimmer/component';

import { addMinutes, subDays as brokenSubDays, subMinutes } from 'date-fns';
import window from 'ember-window-mock';
import semverSort from 'semver/functions/sort';

// Colors by http://colorbrewer2.org/#type=diverging&scheme=RdBu&n=10
const COLORS = ['#67001f', '#b2182b', '#d6604d', '#f4a582', '#92c5de', '#4393c3', '#2166ac', '#053061'];
const BG_COLORS = ['#d3b5bc', '#eabdc0', '#f3d0ca', '#fce4d9', '#deedf5', '#c9deed', '#2166ac', '#053061'];

const ONE_DAY = 24 * 60 * 60 * 1000;

export default class DownloadGraph extends Component {
  @service chartjs;
  @service colorScheme;

  @action loadChartJs() {
    waitForPromise(this.chartjs.loadTask.perform()).catch(() => {
      // Ignore Promise rejections. We'll handle them through the derived state properties.
    });
  }

  get fontColor() {
    return this.colorScheme.isDark ? '#ADBABD' : '#666';
  }

  get borderColor() {
    return this.colorScheme.isDark ? 'rgba(255, 255, 255, 0.1)' : 'rgba(0, 0, 0, 0.1)';
  }

  @action createChart(element) {
    let Chart = this.chartjs.loadTask.lastSuccessful.value;

    let { fontColor: color, borderColor } = this;

    this.chart = new Chart(element, {
      type: 'line',
      data: this.data,
      options: {
        color,
        maintainAspectRatio: false,
        layout: {
          padding: 10,
        },
        scales: {
          x: {
            type: 'time',
            time: { tooltipFormat: 'MMM d', unit: 'day' },
            ticks: { maxTicksLimit: 13, color },
            grid: { color: borderColor },
          },
          y: {
            beginAtZero: true,
            stacked: true,
            ticks: { precision: 0, color },
            grid: { color: borderColor },
          },
        },
        interaction: {
          mode: 'index',
          intersect: false,
          position: 'nearest',
        },
      },
    });
  }

  @action updateColorScheme() {
    let { chart } = this;

    if (chart) {
      let { fontColor, borderColor } = this;
      chart.options.color = fontColor;
      chart.options.scales.x.ticks.color = fontColor;
      chart.options.scales.x.grid.color = borderColor;
      chart.options.scales.y.ticks.color = fontColor;
      chart.options.scales.y.grid.color = borderColor;
      chart.update();
    }
  }

  @action updateChart() {
    let { chart } = this;

    if (chart) {
      chart.data = this.data;
      chart.update();
    }
  }

  @action updateStacked() {
    let { chart, data } = this;

    if (chart) {
      data.dataset = data.datasets.map(d => {
        d.fill = this.args.stacked ? 'origin' : false;
        chart.options.scales.y.stacked = this.args.stacked;
        return d;
      });
      chart.data = data;
      chart.update();
    }
  }

  @action destroyChart() {
    this.chart?.destroy();
  }

  @action reloadPage() {
    window.location.reload();
  }

  get data() {
    return toChartData(this.args.data, this.args.versions);
  }
}

export function toChartData(data) {
  if (!data) {
    return { datasets: [] };
  }

  let extra = data.meta?.extra_downloads ?? [];

  let dates = {};
  let versions = new Map();
  let crate = null;

  let now = new Date();
  for (let i = 0; i < 90; i++) {
    let date = subDays(now, i);
    dates[date.toISOString().slice(0, 10)] = { date, cnt: {} };
  }

  data.forEach(d => {
    let version = d.version;
    if (!version) return;

    let version_num = version.num;

    versions.set(version_num, version);

    if (version.crate) {
      crate = version.crate;
    }

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

  let rows = Object.keys(dates).map(date => [
    dates[date].date,
    ...versionsList.map(version => dates[date].cnt[version] || 0),
  ]);

  let datasets = versionsList
    .map((label, index) => {
      let data = rows.map(row => ({ x: row[0], y: row[index + 1] }));

      // if we find a corresponding version
      let version = versions.get(label);
      if (version?.created_at) {
        // only show downloads from the day before the release until today
        let threshold = midnightForDate(version.created_at) - ONE_DAY;
        data = data.filter(it => midnightForDate(it.x) >= threshold);
      }

      // if we find a corresponding crate
      if (crate?.created_at) {
        // only show downloads from the day before the first release until today
        let threshold = midnightForDate(crate.created_at) - ONE_DAY;
        data = data.filter(it => midnightForDate(it.x) >= threshold);
      }

      return { data, label };
    })
    .reverse()
    .map(({ label, data }, index) => {
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

function midnightForDate(date) {
  return Date.parse(date.toISOString().slice(0, 10));
}

// This works around a bug in date-fn's subDays() function when crossing a DST
// transition: https://github.com/date-fns/date-fns/issues/571
//
// The specific implementation is based on @bertho-zero's here:
// https://github.com/date-fns/date-fns/issues/571#issuecomment-602496322
function subDays(date, amount) {
  const originalTZO = date.getTimezoneOffset();
  const endDate = brokenSubDays(date, amount);
  const endTZO = endDate.getTimezoneOffset();

  const dstDiff = originalTZO - endTZO;

  return dstDiff >= 0 ? addMinutes(endDate, dstDiff) : subMinutes(endDate, Math.abs(dstDiff));
}
