import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

export default defineConfig({
    plugins: [svelte({ hot: false })],
    resolve: {
        conditions: ['browser'],
        alias: {
            $lib: resolve('./src/lib'),
            '@lib': resolve('./src/lib'),
            '@components': resolve('./src/lib/components'),
            '@utils': resolve('./src/lib/utils'),
        },
    },
    test: {
        environment: 'jsdom',
        globals: true,
        include: ['src/**/*.{test,spec}.{js,ts}'],
        setupFiles: ['src/test/setup.ts'],
    },
});
