import { describe, expect, it } from 'vitest';
import { listProviders, startUrl } from './providers';

describe('providers', () => {
	it('lists what the server offers, or nothing when it cannot', async () => {
		const ok = async () => new Response(JSON.stringify([{ id: 'lichess', name: 'Lichess' }]));
		expect(await listProviders(ok)).toEqual([{ id: 'lichess', name: 'Lichess' }]);
		const down = async () => new Response(null, { status: 503 });
		expect(await listProviders(down)).toEqual([]);
		const gone = async () => {
			throw new Error('no server');
		};
		expect(await listProviders(gone)).toEqual([]);
	});

	it('builds the start URL with the return path', () => {
		expect(startUrl({ id: 'lichess', name: 'Lichess' }, '/game/abc?x=1')).toBe(
			'/api/auth/lichess/start?next=%2Fgame%2Fabc%3Fx%3D1'
		);
	});
});
