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
