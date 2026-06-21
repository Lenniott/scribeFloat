import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'node:path';

export default defineConfig({
	plugins: [svelte({ hot: false })],
	resolve: {
		conditions: ['browser'],
		alias: {
			$lib: path.resolve('./src/lib'),
			'@lib': path.resolve('./src/lib'),
			'@ui': path.resolve('./src/lib/ui'),
			'@primitives': path.resolve('./src/lib/ui/1_primitives'),
			'@components': path.resolve('./src/lib/ui/2_components'),
			'@patterns': path.resolve('./src/lib/ui/3_patterns'),
			'@sections': path.resolve('./src/lib/ui/4_sections'),
			'@views': path.resolve('./src/lib/ui/5_views'),
			'@regions': path.resolve('./src/lib/ui/6_regions'),
			'@utils': path.resolve('./src/lib/utils'),
			'@services': path.resolve('./src/lib/services'),
			'@stores': path.resolve('./src/lib/stores'),
		},
	},
	test: {
		environment: 'jsdom',
		globals: true,
		include: ['src/**/*.{test,spec}.{js,ts}'],
		setupFiles: ['src/test/setup.ts'],
	},
});
