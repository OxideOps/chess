/** `4:59` normally, `0:09.3` under ten seconds, never negative. */
export function formatClock(ms: number): string {
	const clamped = Math.max(0, ms);
	if (clamped < 10_000) {
		const tenths = Math.floor(clamped / 100);
		return `0:0${Math.floor(tenths / 10)}.${tenths % 10}`;
	}
	const totalSeconds = Math.floor(clamped / 1000);
	const minutes = Math.floor(totalSeconds / 60);
	const seconds = totalSeconds % 60;
	return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

export interface TimeControl {
	label: string;
	initialMs: number;
	incrementMs: number;
}

export const TIME_CONTROLS: TimeControl[] = [
	{ label: '1+0 Bullet', initialMs: 60_000, incrementMs: 0 },
	{ label: '3+2 Blitz', initialMs: 180_000, incrementMs: 2000 },
	{ label: '5+0 Blitz', initialMs: 300_000, incrementMs: 0 },
	{ label: '10+0 Rapid', initialMs: 600_000, incrementMs: 0 },
	{ label: '15+10 Rapid', initialMs: 900_000, incrementMs: 10_000 }
];
