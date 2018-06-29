import $ from 'jquery';

function decodeFragmentValue(hash) {
    try {
        return decodeURIComponent(hash.slice(1));
    } catch (_) {
        return '';
    }
}

function findElementByFragmentName(document, name) {
    if (name === '') {
        return;
    }

    return document.getElementById(name) || document.getElementsByName(name)[0];
}

function hashchange() {
    if (document.querySelector(':target')) {
        return;
    }

    const hash = decodeFragmentValue(location.hash);
    const target = findElementByFragmentName(document, `user-content-${hash}`);
    if (target) {
        target.scrollIntoView();
    }
}

export function initialize() {
    $(window).on('hashchange', hashchange);

    // If clicking on a link to the same fragment as currently in the address bar,
    // hashchange won't be fired, so we need to manually trigger rescroll.
    $(document).on('a[href]', 'click', function(event) {
        if (this.href === location.href && location.hash.length > 1) {
            setTimeout(function() {
                if (!event.defaultPrevented) {
                    hashchange();
                }
            });
        }
    });
}

export default {
    name: 'app.hashchange',
    initialize,
};
