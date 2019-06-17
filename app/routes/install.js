import Route from '@ember/routing/route';

export default Route.extend({
    redirect() {
        window.location = 'https://doc.rust-lang.org/cargo/getting-started/installation.html';
    },
});
