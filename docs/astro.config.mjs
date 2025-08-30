// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://yourusername.github.io',
	base: '/actor12',
	integrations: [
		starlight({
			title: 'Actor12',
			description: 'A Rust actor framework with ergonomic two-step messaging API',
			social: [
				{
					icon: 'github',
					label: 'GitHub', 
					href: 'https://github.com/yourusername/actor12',
				},
			],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'introduction' },
						{ label: 'Installation', slug: 'installation' },
						{ label: 'Quick Start', slug: 'quick-start' },
					],
				},
				{
					label: 'Core Concepts',
					items: [
						{ label: 'Actors', slug: 'concepts/actors' },
						{ label: 'Messages', slug: 'concepts/messages' },
					],
				},
				{
					label: 'API Guide',
					items: [
						{ label: 'Two-Step API', slug: 'api/two-step' },
					],
				},
				{
					label: 'Examples',
					items: [
						{ label: 'Basic Counter', slug: 'examples/counter' },
					],
				},
				{
					label: 'Migration',
					items: [
						{ label: 'From Old API', slug: 'migration/from-old-api' },
					],
				},
			],
		}),
	],
});
