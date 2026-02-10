<script lang="ts">
	import { lngLatToStr } from '$lib/lngLatToStr';
	import { MapboxOverlay } from '@deck.gl/mapbox';
	import { IconLayer } from '@deck.gl/layers';
	import { createTripIcon } from '$lib/map/createTripIcon';
	import maplibregl from 'maplibre-gl';
	import { onDestroy, onMount, untrack } from 'svelte';
	import { formatTime } from './toDateTime';
	import { onClickTrip } from './utils';
	import { getDelayColor, rgbToHex } from './Color';
	import type { MetaData } from './types';
	import Control from './map/Control.svelte';
	import { client } from '@motis-project/motis-client';
	import type { PickingInfo } from '@deck.gl/core';
	let {
		map,
		bounds,
		zoom,
		colorMode
	}: {
		map: maplibregl.Map | undefined;
		bounds: maplibregl.LngLatBoundsLike | undefined;
		zoom: number;
		colorMode: 'rt' | 'route' | 'mode' | 'none';
	} = $props();

	//QUERY
	let startTime = $state(new Date(Date.now()));
	let endTime = $derived(new Date(startTime.getTime() + 180000));
	let canceled = $derived(colorMode === 'none');
	let query = $derived.by(() => {
		if (!bounds || !zoom) return null;
		const b = maplibregl.LngLatBounds.convert(bounds);
		const max = lngLatToStr(b.getNorthWest());
		const min = lngLatToStr(b.getSouthEast());
		return {
			min,
			max,
			startTime: startTime.toISOString(),
			endTime: endTime.toISOString(),
			zoom
		};
	});

	//TRANSFERABLES
	let isProcessing = false;
	const TRIPS_NUM = 12000;
	const positions = new Float64Array(TRIPS_NUM * 2);
	const angles = new Float32Array(TRIPS_NUM);
	const colors = new Uint8Array(TRIPS_NUM * 3);
	const DATA = {
		length: TRIPS_NUM,
		positions,
		colors,
		angles
	};

	//INTERACTION
	const popup = new maplibregl.Popup({
		closeButton: false,
		closeOnClick: true,
		maxWidth: 'none'
	});
	let activeHoverIndex = $state(-1);
	let clickRequested = -1;

	const onHover = (info: PickingInfo) => {
		activeHoverIndex = info.index;
		if (info.picked && info.index !== -1 && metadata) {
			createPopup(metadata, info.coordinate as maplibregl.LngLatLike);
		} else {
			metadata = undefined;
			popup.remove();
			if (map) map.getCanvas().style.cursor = '';
		}
	};
	const onClick = (info: PickingInfo) => {
		if (info.picked && info.index !== -1) {
			if (info.index != activeHoverIndex || !metadata) {
				metadata = undefined;
				activeHoverIndex = info.index;
				clickRequested = info.index;
			} else if (metadata) {
				onClickTrip(metadata.id);
			}
		}
	};
	const formatPopupTime = (time: string, tz: string | undefined) =>
		formatTime(new Date(time), tz ?? 'UTC');

	const createRealtimeTimeNode = (
		realtimeTime: string,
		scheduledTime: string,
		tz: string | undefined,
		delay: number
	) => {
		const wrapper = document.createDocumentFragment();
		const realtime = document.createElement('span');
		realtime.style.color = rgbToHex(getDelayColor(delay, true));
		realtime.textContent = formatPopupTime(realtimeTime, tz);
		wrapper.append(realtime);
		wrapper.append(document.createTextNode(' '));
		const scheduled = document.createElement('span');
		if (delay !== 0) {
			scheduled.className = 'line-through';
		}
		scheduled.textContent = formatPopupTime(scheduledTime, tz);
		wrapper.append(scheduled);
		return wrapper;
	};

	const createPopupContent = (trip: MetaData): HTMLElement => {
		const container = document.createElement('div');
		const title = document.createElement('strong');
		title.textContent = trip.displayName ?? 'Trip';
		container.append(title);
		container.append(document.createElement('br'));

		if (trip.realtime) {
			container.append(
				createRealtimeTimeNode(
					trip.departure,
					trip.scheduledDeparture,
					trip.tz,
					trip.departureDelay
				),
				document.createTextNode(` ${trip.from}`),
				document.createElement('br'),
				createRealtimeTimeNode(trip.arrival, trip.scheduledArrival, trip.tz, trip.arrivalDelay),
				document.createTextNode(` ${trip.to}`)
			);
		} else {
			container.append(
				document.createTextNode(`${formatPopupTime(trip.departure, trip.tz)} ${trip.from}`),
				document.createElement('br'),
				document.createTextNode(`${formatPopupTime(trip.arrival, trip.tz)} ${trip.to}`)
			);
		}
		return container;
	};

	const createPopup = (trip: MetaData, hoverCoordinate: maplibregl.LngLatLike) => {
		if (!trip || !map) return;

		map.getCanvas().style.cursor = 'pointer';
		popup.setLngLat(hoverCoordinate).setDOMContent(createPopupContent(trip)).addTo(map);
	};

	//ANIMATION
	const TripIcon = createTripIcon(128);
	const IconMapping = {
		marker: {
			x: 0,
			y: 0,
			width: 128,
			height: 128,
			anchorY: 64,
			anchorX: 64,
			mask: true
		}
	};
	const createLayer = () => {
		if (!DATA.positions || DATA.positions.byteLength === 0) return;
		return new IconLayer({
			id: 'trips-layer',
			data: {
				length: DATA.length,
				attributes: {
					getPosition: { value: DATA.positions, size: 2 },
					getAngle: { value: DATA.angles, size: 1 },
					getColor: { value: DATA.colors, size: 3, normalized: true }
				}
			},
			beforeId: 'road-name-text',
			// @ts-expect-error: canvas element seems to work fine
			iconAtlas: TripIcon,
			iconMapping: IconMapping,
			pickable: colorMode != 'none',
			sizeScale: 5,
			getSize: 10,
			getIcon: (_) => 'marker',
			colorFormat: 'RGB',
			visible: colorMode !== 'none',
			useDevicePixels: false,
			parameters: { depthTest: false },
			onClick,
			onHover
		});
	};
	let animationId: number;
	const animate = () => {
		if (!DATA.positions || DATA.positions.length === 0) return;
		worker.postMessage(
			{
				type: 'update',
				colorMode,
				positions: DATA.positions,
				index: activeHoverIndex,
				angles: DATA.angles,
				colors: DATA.colors,
				length: DATA.length
			},
			[DATA.positions.buffer, DATA.angles.buffer, DATA.colors.buffer]
		);
	};

	// UPDATE
	$effect(() => {
		if (!query || isProcessing || canceled) return;
		untrack(() => {
			isProcessing = true;
			worker.postMessage({ type: 'fetch', query });
		});
	});
	setInterval(() => {
		if (query && colorMode !== 'none') {
			startTime = new Date();
		}
	}, 60000);

	//SETUP
	let status = $state();
	let overlay: MapboxOverlay;
	let worker: Worker;
	let metadata: MetaData | undefined = $state();

	onMount(() => {
		worker = new Worker(new URL('tripsWorker.ts', import.meta.url), { type: 'module' });
		worker.postMessage({ type: 'init', baseUrl: client.getConfig().baseUrl });
		worker.onmessage = (e) => {
			if (e.data.type == 'fetch-complete') {
				status = e.data.status;
				activeHoverIndex = -1;
				metadata = undefined;
				isProcessing = false;
			} else {
				const { positions, angles, length, colors } = e.data;
				DATA.positions = new Float64Array(positions.buffer);
				DATA.angles = new Float32Array(angles.buffer);
				DATA.colors = new Uint8Array(colors.buffer);
				DATA.length = length;
				metadata = e.data.metadata;
				if (clickRequested != -1 && e.data.metadataIndex == clickRequested && metadata) {
					onClickTrip(metadata.id);
					clickRequested = -1;
				}
			}
			overlay.setProps({ layers: [createLayer()] });
			if (canceled) {
				cancelAnimationFrame(animationId);
			} else {
				animationId = requestAnimationFrame(animate);
			}
		};
		overlay = new MapboxOverlay({ interleaved: true });
	});
	$effect(() => {
		if (!map || !overlay) return;
		map.addControl(overlay);
	});
	onDestroy(() => {
		if (animationId) cancelAnimationFrame(animationId);
		if (overlay) map?.removeControl(overlay);
		worker.terminate();
		popup.remove();
	});
</script>

{#if status && status !== 200}
	<Control position="bottom-left">trips response status: {status}</Control>
{/if}
