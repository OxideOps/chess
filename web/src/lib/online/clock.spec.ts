import { describe, expect, it } from 'vitest';
import { formatClock } from './clock';

describe('formatClock', () => {
	it('shows minutes and seconds, then tenths under ten seconds', () => {
		expect(formatClock(300_000)).toBe('5:00');
		expect(formatClock(299_999)).toBe('4:59');
		expect(formatClock(61_000)).toBe('1:01');
		expect(formatClock(10_000)).toBe('0:10');
		expect(formatClock(9_999)).toBe('0:09.9');
		expect(formatClock(1_050)).toBe('0:01.0');
		expect(formatClock(0)).toBe('0:00.0');
		expect(formatClock(-500)).toBe('0:00.0');
	});
});
