// How ratings read on screen.
import type { Category } from '$lib/generated/Category';
import type { PlayerRating } from '$lib/generated/PlayerRating';

export const CATEGORY_NAMES: Record<Category, string> = {
	bullet: 'Bullet',
	blitz: 'Blitz',
	rapid: 'Rapid',
	classical: 'Classical'
};

/** "1512", or "1500?" while the rating is still a guess. */
export function formatRating(rating: PlayerRating | null): string | null {
	if (rating === null) return null;
	return `${rating.value}${rating.provisional ? '?' : ''}`;
}

/** "+12", "−12" (a real minus sign), or "±0". */
export function formatDiff(diff: number): string {
	if (diff > 0) return `+${diff}`;
	if (diff < 0) return `−${-diff}`;
	return '±0';
}

/** "Rated · Blitz" or "Casual · Rapid". */
export function gameKind(rated: boolean, category: Category): string {
	return `${rated ? 'Rated' : 'Casual'} · ${CATEGORY_NAMES[category]}`;
}
