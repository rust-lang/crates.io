import Ember from 'ember';

export default Ember.Component.extend({
    classNames: 'graph-data',

    didInsertElement() {
        this._super(...arguments);

        Ember.$(window).on('resize.chart', () => this.rerender());
        Ember.$(document).on('googleChartsLoaded', () => this.rerender());
    },

    willDestroyElement() {
        Ember.$(window).off('resize.chart');
        Ember.$(document).off('googleChartsLoaded');
    },

    didRender() {
        this._super(...arguments);

        let data = this.get('data');

        let datapoints_count = (data[1] || []).length - 1;

        // "on" is an array of booleans where each
        // element in the array says whether or not the
        // corresponding line in the data is being drawn or not.
        //
        // Each line starts as not being drawn, and can pick up and
        // be stopped arbitrarily many times before the graph is complete.
        let on = [];
        for (let i = 0; i < datapoints_count; i++) {
            on.push(false);
        }

        // Start at 1 because the 0th entry in the array
        // is an array of version numbers
        for (let i = 1; i < data.length; i++) {
            for (let k = 0; k < datapoints_count; k++) {
                // k + 1 because the first entry in the array is the date
                let value = data[i][k + 1];

                // If we are "off" and are looking at a zero
                // replace the data at this point with `null`.
                //
                // Null tells google.visualization to stop drawing
                // the line altogether.
                if (!on[k] && value === 0) {
                    data[i][k + 1] = null;
                }
                // If we are off and the value is not zero, we
                // need to turn back on.  (keep the value the same though)
                else if (!on[k] && value !== 0) {
                    on[k] = true;

                    // We previously wrote a null into data[i - 1][k + 1],
                    // so to make the graph look pretty, we'll switch it back
                    // to the zero that it was before.
                    if (i > 1) {
                        data[i - 1][k + 1] = 0;
                    }
                }
                // If we are on and the value is zero, turn off
                // but keep the zero in the array
                else if (on[k] && value === 0) {
                    on[k] = false;
                }
            }
        }

        if (!data || !window.google || !window.googleChartsLoaded) {
            this.$().hide();
            return;
        } else {
            this.$().show();
        }

        var myData = window.google.visualization.arrayToDataTable(data);

        var fmt = new window.google.visualization.DateFormat({
            pattern: 'LLL d, yyyy',
        });
        fmt.format(myData, 0);

        var chart = new window.google.visualization.AreaChart(this.get('element'));
        chart.draw(myData, {
            chartArea: { 'left': 85, 'width': '77%', 'height': '80%' },
            hAxis: {
                minorGridlines: { count: 8 },
            },
            vAxis: {
                minorGridlines: { count: 5 },
                viewWindow: { min: 0, },
            },
            isStacked: true,
            focusTarget: 'category',
        });
    },
});
