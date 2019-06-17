import Component from '@ember/component';

// Colors by http://colorbrewer2.org/#type=diverging&scheme=RdBu&n=10
const COLORS = ['#67001f', '#b2182b', '#d6604d', '#f4a582', '#92c5de', '#4393c3', '#2166ac', '#053061'];

export default Component.extend({
    classNames: 'graph-data',
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

        let subarray_length = (data[1] || []).length;

        // Start at 1 to skip the date element in the 0th
        // location in the array.
        for (let k = 1; k < subarray_length; k++) {
            let on = false;

            // Start at 1 because the 0th entry in the array
            // is an array of version numbers.
            //
            // End before the last element is reached because we never
            // want to change the last element.
            for (let i = 1; i < data.length - 1; i++) {
                // k + 1 because the first entry in the array is the date
                let value = data[i][k];

                // If we are "off" and are looking at a zero
                // replace the data at this point with `null`.
                //
                // Null tells google.visualization to stop drawing
                // the line altogether.
                if (!on && value === 0) {
                    data[i][k] = null;
                }

                // If we are off and the value is not zero, we
                // need to turn back on.  (keep the value the same though)
                else if (!on && value !== 0) {
                    on = true;

                    // We previously wrote a null into data[i - 1][k + 1],
                    // so to make the graph look pretty, we'll switch it back
                    // to the zero that it was before.
                    if (i >= 2) {
                        data[i - 1][k] = 0;
                    }
                }
                // If we are on and the value is zero, turn off
                // but keep the zero in the array
                else if (on && value === 0) {
                    on = false;
                }
            }
        }

        let show = data && window.google && window.googleChartsLoaded;
        this.element.style.display = show ? '' : 'none';
        if (!show) {
            return;
        }

        let myData = window.google.visualization.arrayToDataTable(data);

        let dateFmt = new window.google.visualization.DateFormat({
            pattern: 'LLL d, yyyy',
        });
        dateFmt.format(myData, 0);

        // Create a formatter to use for daily download numbers
        let numberFormatWhole = new window.google.visualization.NumberFormat({
            pattern: '#,##0',
        });

        // Create a formatter to use for 7-day average numbers
        let numberFormatDecimal = new window.google.visualization.NumberFormat({
            pattern: '#,##0.0',
        });

        // use a DataView to calculate an x-day moving average
        let days = 7;
        let view = new window.google.visualization.DataView(myData);
        let moving_avg_func_for_col = function(col) {
            return function(dt, row) {
                // For the last rows (the *first* days, remember, the dataset is
                // backwards), we cannot calculate the avg. of previous days.
                if (row >= dt.getNumberOfRows() - days) {
                    return null;
                }

                let total = 0;
                for (let i = days; i > 0; i--) {
                    total += dt.getValue(row + i, col);
                }
                let avg = total / days;
                return {
                    v: avg,
                    f: numberFormatDecimal.formatValue(avg),
                };
            };
        };

        let columns = [0]; // 0 = datetime
        let seriesOption = {};
        let [headers] = data;
        // Walk over the headers/colums in reverse order, as the newest version
        // is at the end, but in the UI we want it at the top of the chart legend.

        range(headers.length - 1, 0, -1).forEach((dataCol, i) => {
            // Set the number format for the colum in the data table.
            numberFormatWhole.format(myData, dataCol);
            columns.push(dataCol); // add the column itself
            columns.push({
                // add a 'calculated' column, the moving average
                type: 'number',
                label: `${headers[dataCol]} ${days}-day avg.`,
                calc: moving_avg_func_for_col(dataCol),
            });
            // Note: while the columns start with index 1 (because 0 is the time
            // axis), the series configuration starts with index 0.
            seriesOption[i * 2] = {
                type: 'scatter',
                color: COLORS[i % COLORS.length],
                pointSize: 3,
                pointShape: 'square',
            };
            seriesOption[i * 2 + 1] = {
                type: 'area',
                color: COLORS[i % COLORS.length],
                lineWidth: 2,
                curveType: 'function',
                visibleInLegend: false,
            };
        });
        view.setColumns(columns);

        let chart = new window.google.visualization.ComboChart(this.element);
        chart.draw(view, {
            chartArea: { left: 85, width: '77%', height: '80%' },
            hAxis: {
                minorGridlines: { count: 8 },
            },
            vAxis: {
                minorGridlines: { count: 5 },
                viewWindow: { min: 0 },
            },
            isStacked: true,
            focusTarget: 'category',
            seriesType: 'scatter',
            series: seriesOption,
        });
    },
});

function range(start, end, step) {
    let array = [];
    for (let i = start; i !== end; i += step) {
        array.push(i);
    }
    return array;
}
