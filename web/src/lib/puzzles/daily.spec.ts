import { describe, expect, it } from 'vitest';
import { formatDay, isDay } from './daily';

describe('the daily puzzle date', () => {
	it('takes only YYYY-MM-DD', () => {
		expect(isDay('2026-09-22')).toBe(true);
		expect(isDay(null)).toBe(false);
		expect(isDay('2026-9-22')).toBe(false);
		expect(isDay('today')).toBe(false);
		expect(isDay('2026-09-22&x=1')).toBe(false);
	});

	it('reads the day in UTC, whatever the local time zone', () => {
		expect(formatDay('2026-09-22')).toBe('22 September 2026');
		expect(formatDay('2027-01-01')).toBe('1 January 2027');
	});
});
