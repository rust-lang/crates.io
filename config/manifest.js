'use strict';

module.exports = function(/* environment, appConfig */) {
    return {
        name: 'Cargo: packages for Rust',
        short_name: 'Cargo',
        description: 'Cargo is the package manager and crate host for Rust.',
        start_url: '/',
        display: 'standalone',
        background_color: '#3b6837',
        theme_color: '#f9f7ec',
        icons: [
            {
                src: 'cargo.png',
                sizes: '227x227',
                type: 'image/png',
            },
        ],
        ms: {
            tileColor: '#3b6837',
        },
    };
};
