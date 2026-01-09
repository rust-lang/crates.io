import type { Chart as ChartType, TimeUnit } from 'chart.js';

import { addDays, differenceInDays, endOfDay, format, getTime, startOfDay } from 'date-fns';

let chartPromise: Promise<typeof ChartType> | null = null;

/**
 * Lazily loads Chart.js and registers required components.
 * Returns a singleton promise that resolves to the Chart constructor.
 */
export function loadChart(): Promise<typeof ChartType> {
  if (!chartPromise) {
    chartPromise = (async () => {
      let {
        Chart,
        LineController,
        LineElement,
        PointElement,
        TimeScale,
        LinearScale,
        Filler,
        Legend,
        Tooltip,
        _adapters,
      } = await import('chart.js');

      Chart.register(LineController, LineElement, PointElement, LinearScale, TimeScale, Filler, Legend, Tooltip);

      _adapters._date.override({
        formats() {
          return { day: 'MMM d' } as Record<TimeUnit | 'datetime', string>;
        },

        parse(value) {
          if (value === null || value === undefined) {
            return null;
          }
          assert('`value` must be a `Date`', value instanceof Date);
          let time = value.getTime();
          return isNaN(time) ? null : time;
        },

        format(time, fmt) {
          return format(time, fmt);
        },

        add(time, amount, unit) {
          assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
          return getTime(addDays(time, amount));
        },

        diff(max, min, unit) {
          assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
          return differenceInDays(max, min);
        },

        startOf(time, unit) {
          assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
          return getTime(startOfDay(time));
        },

        endOf(time, unit) {
          assert('This basic Chart.js adapter only supports `unit: day`', unit === 'day');
          return getTime(endOfDay(time));
        },
      });

      return Chart;
    })();
  }

  return chartPromise;
}

function assert(message: string, predicate: boolean): asserts predicate {
  if (import.meta.env.DEV && !predicate) {
    throw new Error(message);
  }
}
