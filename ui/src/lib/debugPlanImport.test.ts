import { describe, expect, it } from 'vitest';
import { parseDebugPlanResponse } from '$lib/debugPlanImport';

describe('parseDebugPlanResponse', () => {
	it('returns undefined for non-JSON clipboard content', () => {
		expect(parseDebugPlanResponse('hello')).toBeUndefined();
	});

	it('returns undefined for JSON that is not a plan response', () => {
		expect(parseDebugPlanResponse('{"foo":"bar"}')).toBeUndefined();
	});

	it('returns a plan response when itineraries is present', () => {
		const parsed = parseDebugPlanResponse('{"itineraries":[]}');
		expect(parsed).toBeDefined();
		expect(parsed?.itineraries).toEqual([]);
	});
});
