<script lang="ts">
	import '../app.css';
	import { browser } from '$app/environment';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import { onMount } from 'svelte';

	const { children } = $props();

	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				enabled: browser
			}
		}
	});

	const isInMapContainer = (target: EventTarget | null): boolean =>
		target instanceof Element && target.closest('.motis-map') !== null;

	onMount(() => {
		const onWheel = (event: WheelEvent) => {
			if (!event.ctrlKey && !event.metaKey) {
				return;
			}
			if (isInMapContainer(event.target)) {
				return;
			}
			event.preventDefault();
		};

		const onKeyDown = (event: KeyboardEvent) => {
			if (!event.ctrlKey && !event.metaKey) {
				return;
			}
			if (
				event.key === '+' ||
				event.key === '=' ||
				event.key === '-' ||
				event.key === '_' ||
				event.key === '0'
			) {
				event.preventDefault();
			}
		};

		const onGesture = (event: Event) => {
			if (isInMapContainer(event.target)) {
				return;
			}
			event.preventDefault();
		};

		window.addEventListener('wheel', onWheel, { passive: false, capture: true });
		window.addEventListener('keydown', onKeyDown, { capture: true });
		window.addEventListener('gesturestart', onGesture, { passive: false, capture: true });
		window.addEventListener('gesturechange', onGesture, { passive: false, capture: true });
		window.addEventListener('gestureend', onGesture, { passive: false, capture: true });

		return () => {
			window.removeEventListener('wheel', onWheel, { capture: true });
			window.removeEventListener('keydown', onKeyDown, { capture: true });
			window.removeEventListener('gesturestart', onGesture, { capture: true });
			window.removeEventListener('gesturechange', onGesture, { capture: true });
			window.removeEventListener('gestureend', onGesture, { capture: true });
		};
	});
</script>

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>
