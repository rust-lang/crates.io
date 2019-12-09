import semver from 'semver';
import Component from '@ember/component';

// Component to plot downloadeds split by semver minor version.
export default Component.extend({
  resizeHandler: undefined,

  didInsertElement() {
    this._super(...arguments);

    this.resizeHandler = () => this.rerender();
    window.addEventListener('resize', this.resizeHandler, false);
    document.addEventListener('googleChartsLoaded', this.resizeHandler, false);
  },

  willDestroyElement() {
    window.removeEventListener('resize', this.resizeHandler);
    document.removeEventListener('googleChartsLoaded', this.resizeHandler);
  },

  didRender() {
    this._super(...arguments);

    let data = this.data;

    // Early exit if the google plotting libraries do not exist
    let show = data && window.google && window.googleChartsLoaded;
    this.element.style.display = show ? '' : 'none';
    if (!show) {
      console.error('cannot show graph due to missing google dependencies');
      return;
    }

    // Build up the list of unique major.minor versions of this crate, and total
    // up the number of downloads for each semver version
    let downloadsPerVersion = new Map();
    data.forEach(v => {
      let major = semver.major(v.num);
      let minor = semver.minor(v.num);
      let downloads = v.downloads;

      // XXX ugly hack to get semver to parse the version correctly later on.
      // We want to do a semver-aware sort, but the `semver.lt` function only
      // understands version triples, not doubles.
      let key = `${major}.${minor}.0`;
      if (downloadsPerVersion.has(key)) {
        const old = downloadsPerVersion.get(key);
        downloadsPerVersion.set(key, old + downloads);
      } else {
        downloadsPerVersion.set(key, downloads);
      }
    });

    // Build up the plotting data
    let plotData = [
      // Headings and the nature of additional parameters
      ['Version', 'Downloads', { role: 'style' }, { role: 'annotation' }],
    ];

    // Update plotData with rows in the correct format for google visualization library
    for (var [key, value] of sortIncreasingBySemver(downloadsPerVersion)) {
      plotData.push([key, value, '#62865f', value]);
    }

    let myData = window.google.visualization.arrayToDataTable(plotData);

    // Plot options
    let options = {
      chart: {
        title: 'Downloads',
      },
      chartArea: { left: 85, width: '77%', height: '80%' },
      hAxis: {
        minorGridlines: { count: 8 },
      },
      vAxis: {
        minorGridlines: { count: 5 },
        viewWindow: { min: 0 },
      },
      legend: { position: 'none' },
    };

    // Draw the plot into the current element
    let chart = new window.google.visualization.BarChart(this.element);
    chart.draw(myData, options);
  },
});

function sortIncreasingBySemver(downloadsMap) {
  const items = Array.from(downloadsMap.entries());
  // Sort by semver comparison
  items.sort((a, b) => {
    // Index 0 is the version string in the array.
    //
    // We use `lt` here as we want the array to be sorted in reverse order
    // (newest at the top)
    return semver.lt(a[0], b[0]);
  });

  // Update the labels to show e.g. `0.1.x` instead of `0.1.0` which is
  // required by semver comparisons
  return items.map(i => {
    return [i[0].replace(/\.0$/, '.x'), i[1]];
  });
}
