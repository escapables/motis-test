import type { PlanResponse } from '@motis-project/motis-client';

const hasItinerariesArray = (value: unknown): value is { itineraries: unknown[] } => {
	return Boolean(
		value &&
			typeof value === 'object' &&
			'itineraries' in value &&
			Array.isArray((value as { itineraries: unknown[] }).itineraries)
	);
};

export const parseDebugPlanResponse = (clipboardText: string): PlanResponse | undefined => {
	if (!clipboardText.trim()) {
		return undefined;
	}
	try {
		const parsed: unknown = JSON.parse(clipboardText);
		return hasItinerariesArray(parsed) ? (parsed as PlanResponse) : undefined;
	} catch {
		return undefined;
	}
};
