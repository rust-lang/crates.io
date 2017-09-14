import Route from '@ember/routing/route';

export default Route.extend({
    redirect() {
        window.location = 'http://doc.crates.io/';
    },
});
