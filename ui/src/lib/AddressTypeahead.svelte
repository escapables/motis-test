<script lang="ts">
	import { Combobox } from 'bits-ui';
	import { geocode, type LocationType, type Match, type Mode } from '@motis-project/motis-client';
	import { LoaderCircle } from '@lucide/svelte';
	import { parseCoordinatesToLocation, type Location } from './Location';
	import { getLanguage } from './i18n/translation';
	import maplibregl from 'maplibre-gl';
	import { getModeStyle, type LegLike } from './modeStyle';

	let {
		items = $bindable([]),
		selected = $bindable(),
		placeholder,
		name,
		place,
		type,
		allowCoordinateInput = true,
		matchFilter = () => true,
		transitModes,
		onChange = () => {}
	}: {
		items?: Array<Location>;
		selected: Location;
		placeholder?: string;
		name?: string;
		place?: maplibregl.LngLatLike;
		type?: undefined | LocationType;
		allowCoordinateInput?: boolean;
		matchFilter?: (match: Match) => boolean;
		transitModes?: Mode[];
		onChange?: (location: Location) => void;
	} = $props();

	let inputValue = $state('');
	let match = $state('');
	let loading = $state(false);
	let lookupError = $state<string | undefined>(undefined);

	const MODE_PRIORITY: Partial<Record<Mode, number>> = {
		HIGHSPEED_RAIL: 120,
		LONG_DISTANCE: 115,
		NIGHT_RAIL: 110,
		REGIONAL_FAST_RAIL: 105,
		REGIONAL_RAIL: 100,
		RAIL: 95,
		SUBURBAN: 90,
		SUBWAY: 85,
		TRAM: 80,
		BUS: 70,
		COACH: 65,
		FERRY: 60,
		AIRPLANE: 55,
		TRANSIT: 50
	};

	const getPrimaryMode = (modes?: Mode[]): Mode | undefined => {
		if (!modes || modes.length === 0) return undefined;
		return modes.slice().sort((a, b) => (MODE_PRIORITY[b] ?? 0) - (MODE_PRIORITY[a] ?? 0))[0];
	};

	const isTransitCategory = (category?: string): boolean => {
		if (!category || category === 'none') return false;
		const c = category.toLowerCase();
		return /(bus|rail|train|station|subway|metro|tram|ferry|harbour|harbor|port|terminal|airport|aerodrome|platform|stop)/.test(
			c
		);
	};

	const getModeEmoji = (mode?: Mode): string => {
		switch (mode) {
			case 'HIGHSPEED_RAIL':
			case 'LONG_DISTANCE':
			case 'NIGHT_RAIL':
			case 'REGIONAL_FAST_RAIL':
			case 'REGIONAL_RAIL':
			case 'RAIL':
			case 'SUBURBAN':
				return '🚆';
			case 'SUBWAY':
				return '🚇';
			case 'TRAM':
				return '🚊';
			case 'BUS':
			case 'COACH':
				return '🚌';
			case 'FERRY':
				return '⛴️';
			case 'AIRPLANE':
				return '✈️';
			default:
				return '🚉';
		}
	};

	const getCategoryEmoji = (category?: string): string => {
		if (!category || category === 'none') return '📍';
		const c = category.toLowerCase();
		if (/(airport|aerodrome|helipad)/.test(c)) return '✈️';
		if (/(rail|train|station|subway|metro|tram|platform)/.test(c)) return '🚉';
		if (/bus/.test(c)) return '🚌';
		if (/(ferry|harbour|harbor|port|slipway|boat)/.test(c)) return '⛴️';
		if (/(restaurant|cafe|coffee|bar|pub|fast_food|food)/.test(c)) return '🍽️';
		if (/parking/.test(c)) return '🅿️';
		if (/(hospital|doctors|dentist|pharmacy|medical|veterinary)/.test(c)) return '🏥';
		if (/(hotel|hostel|motel|guest_house)/.test(c)) return '🏨';
		if (/(shop|market|mall|supermarket|bakery|books|clothes)/.test(c)) return '🛍️';
		if (/(park|playground|garden|camping|beach)/.test(c)) return '🌳';
		return '📍';
	};

	const getMatchEmoji = (match: Match | undefined): string => {
		if (!match) return '📍';
		if (match.type === 'STOP') return getModeEmoji(getPrimaryMode(match.modes));
		if (match.type === 'ADDRESS') return '🏠';
		return getCategoryEmoji(match.category);
	};

	const getMatchPriority = (m: Match): number => {
		let p = 0;
		if (m.type === 'STOP') p += 10000;
		if (m.type === 'PLACE' && isTransitCategory(m.category)) p += 6000;
		if (m.type === 'ADDRESS') p += 2000;
		p += MODE_PRIORITY[getPrimaryMode(m.modes) ?? 'OTHER'] ?? 0;
		p += Math.round((m.importance ?? 0) * 100);
		p += Math.round(m.score ?? 0);
		return p;
	};

	const getDisplayArea = (match: Match | undefined) => {
		if (match) {
			const matchedArea = match.areas.find((a) => a.matched);
			const defaultArea = match.areas.find((a) => a.default);
			if (matchedArea?.name.match(/^[0-9]*$/)) {
				matchedArea.name += ' ' + defaultArea?.name;
			}
			let area = (matchedArea ?? defaultArea)?.name;
			if (area == match.name) {
				area = match.areas[0]!.name;
			}

			/* eslint-disable-next-line svelte/prefer-svelte-reactivity */
			const areas = new Set<number>();
			match.areas.forEach((a, i) => {
				if (a.matched || a.unique || a.default) {
					areas.add(i);
				}
			});

			const sorted = Array.from(areas);
			sorted.sort((a, b) => b - a);

			return sorted.map((a) => match.areas[a].name).join(', ');
		}
		return '';
	};

	const getLabel = (match: Match) => {
		const displayArea = getDisplayArea(match);
		return displayArea ? match.name + ', ' + displayArea : match.name;
	};

	const normalizeLocationName = (name?: string): string => {
		if (!name) return '';
		return name
			.toLowerCase()
			.normalize('NFKD')
			.replace(/[\u0300-\u036f]/g, '')
			.replace(/[^a-z0-9]/g, '');
	};

	const getDefaultAreaName = (match: Match): string => {
		const defaultArea = match.areas.find((a) => a.default)?.name;
		return defaultArea ?? match.areas[0]?.name ?? '';
	};

	const STOP_UPGRADE_MAX_DISTANCE_METERS = 750;

	const isSamePlaceAsStop = (place: Match, stop: Match): boolean => {
		if (place.type !== 'PLACE' || stop.type !== 'STOP') return false;

		const placeName = normalizeLocationName(place.name);
		const stopName = normalizeLocationName(stop.name);
		if (!placeName || placeName !== stopName) return false;

		const placeArea = getDefaultAreaName(place);
		const stopArea = getDefaultAreaName(stop);
		if (placeArea && stopArea && placeArea !== stopArea) return false;

		const placePos = new maplibregl.LngLat(place.lon, place.lat);
		const stopPos = new maplibregl.LngLat(stop.lon, stop.lat);
		return placePos.distanceTo(stopPos) <= STOP_UPGRADE_MAX_DISTANCE_METERS;
	};

	const preferEquivalentStop = (loc: Location): Location => {
		const selectedMatch = loc.match;
		if (!selectedMatch || selectedMatch.type !== 'PLACE') {
			return loc;
		}

		const bestStop = items.find(
			(candidate) => candidate.match && isSamePlaceAsStop(selectedMatch, candidate.match)
		)?.match;

		if (!bestStop) {
			return loc;
		}

		return {
			match: bestStop,
			label: getLabel(bestStop)
		};
	};

	const updateGuesses = async () => {
		loading = true;
		lookupError = undefined;

		const coord = allowCoordinateInput ? parseCoordinatesToLocation(inputValue) : undefined;
		if (coord) {
			selected = coord;
			items = [];
			onChange(selected);
			loading = false;
			return;
		}

		try {
			const pos = place ? maplibregl.LngLat.convert(place) : undefined;
			const biasPlace = pos ? { place: `${pos.lat},${pos.lng}` } : {};
			const { data: matches, error } = await geocode({
				query: {
					...biasPlace,
					text: inputValue,
					language: [getLanguage()],
					mode: transitModes,
					type
				}
			});
			if (error) {
				const message = (error as { error?: string })?.error ?? 'Search request failed';
				lookupError = message;
				items = [];
				console.error('TYPEAHEAD ERROR: ', error);
				return;
			}

			items = matches!
				.filter((match: Match) => matchFilter(match))
				.map((match: Match, i: number) => ({ match, i }))
				.sort((a, b) => {
					const d = getMatchPriority(b.match) - getMatchPriority(a.match);
					if (d !== 0) return d;
					return a.i - b.i;
				})
				.map(({ match }): Location => {
					return {
						label: getLabel(match),
						match
					};
				});
			/* eslint-disable-next-line svelte/prefer-svelte-reactivity */
			const shown = new Set<string>();
			items = items.filter((x) => {
				const entry = x.match?.type + x.label!;
				if (shown.has(entry)) {
					return false;
				}
				shown.add(entry);
				return true;
			});
		} catch (error) {
			lookupError = String(error);
			items = [];
		} finally {
			loading = false;
		}
	};

	const deserialize = (s: string): Location => {
		const x = JSON.parse(s);
		return {
			match: x,
			label: getLabel(x)
		};
	};

	$effect(() => {
		if (selected) {
			match = JSON.stringify(selected.match);
			inputValue = selected.label!;
		}
	});

	let ref = $state<HTMLElement | null>(null);
	$effect(() => {
		if (ref && inputValue) {
			(ref as HTMLInputElement).value = inputValue;
		}
	});

	let timer: number;
	$effect(() => {
		if (!inputValue) {
			items = [];
			lookupError = undefined;
			loading = false;
			return;
		}

		if (inputValue) {
			clearTimeout(timer);
			timer = setTimeout(() => {
				updateGuesses();
			}, 150);
		}
	});
</script>

{#snippet modeCircle(mode: Mode)}
	{@const modeIcon = getModeStyle({ mode } as LegLike)[0]}
	{@const modeColor = getModeStyle({ mode } as LegLike)[1]}
	<div
		class="rounded-full flex items-center justify-center p-1"
		style="background-color: {modeColor}; fill: white;"
	>
		<svg class="relative size-4 rounded-full">
			<use xlink:href={`#${modeIcon}`}></use>
		</svg>
	</div>
{/snippet}

<Combobox.Root
	type="single"
	allowDeselect={false}
	value={match}
	onValueChange={(e: string) => {
		if (e) {
			selected = preferEquivalentStop(deserialize(e));
			inputValue = selected.label!;
			onChange(selected);
		}
	}}
>
	<div class="relative">
		<Combobox.Input
			{placeholder}
			{name}
			bind:ref
			class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 pr-9 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
			autocomplete="off"
			oninput={(e: Event) => (inputValue = (e.currentTarget as HTMLInputElement).value)}
			aria-label={placeholder}
			data-combobox-input={inputValue}
		/>
		{#if loading}
			<div
				class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-muted-foreground"
			>
				<LoaderCircle class="h-4 w-4 animate-spin" />
			</div>
		{/if}
	</div>
	{#if lookupError}
		<p class="mt-2 text-xs text-destructive">{lookupError}</p>
	{/if}
	{#if items.length !== 0}
		<Combobox.Portal>
			<Combobox.Content
				align="start"
				class="absolute top-2 w-[var(--bits-combobox-anchor-width)] z-10 overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md outline-none"
			>
				{#each items as item (item.match)}
					<Combobox.Item
						class="flex w-full cursor-default select-none rounded-sm py-4 pl-4 pr-2 text-sm outline-none data-[disabled]:pointer-events-none data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground data-[disabled]:opacity-50"
						value={JSON.stringify(item.match)}
						label={item.label}
					>
						<div class="flex items-center grow">
							<span class="inline-flex h-6 w-6 items-center justify-center text-base leading-none">
								{getMatchEmoji(item.match)}
							</span>
							<div class="flex flex-col ml-4">
								<span class="font-semibold text-nowrap text-ellipsis overflow-hidden">
									{item.match?.name}
								</span>
								<span class="text-muted-foreground text-nowrap text-ellipsis overflow-hidden">
									{getDisplayArea(item.match)}
								</span>
							</div>
						</div>
						{#if item.match?.type == 'STOP'}
							<div class="mt-1 ml-4 flex flex-row gap-1.5 items-center">
								{#each item.match.modes! as mode, i (i)}
									{@render modeCircle(mode)}
								{/each}
							</div>
						{/if}
					</Combobox.Item>
				{/each}
			</Combobox.Content>
		</Combobox.Portal>
	{/if}
</Combobox.Root>
