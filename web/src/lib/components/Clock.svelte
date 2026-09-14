<script lang="ts">
	import { sideName } from '$lib/chess/status';
	import { formatClock } from '$lib/online/clock';
	import { formatDiff, formatRating } from '$lib/online/ratings';
	import type { ResolvedPathname } from '$app/types';
	import type { PlayerRating } from '$lib/generated/PlayerRating';
	import type { Side } from '$lib/generated/Side';

	// One side's remaining time and who is playing it. `active` while that
	// side's clock is running.
	interface Props {
		ms: number;
		side: Side;
		/** The player's display name; `null` while the seat is open. */
		name?: string | null;
		/** Their profile, for registered players. */
		href?: ResolvedPathname | null;
		rating?: PlayerRating | null;
		/** What the finished game did to their rating. */
		diff?: number | null;
		active?: boolean;
	}
	let {
		ms,
		side,
		name = null,
		href = null,
		rating = null,
		diff = null,
		active = false
	}: Props = $props();
	const shownRating = $derived(formatRating(rating));
	const low = $derived(ms < 20_000);
</script>

<div class="clock" class:active class:low aria-label="{sideName(side)} clock">
	<span class="who">
		<span class="name-line">
			{#if href && name}
				<a class="name" {href}>{name}</a>
			{:else}
				<span class="name" class:open={name === null}>{name ?? 'Open seat'}</span>
			{/if}
			{#if shownRating}
				<span class="rating" data-testid="rating">{shownRating}</span>
			{/if}
			{#if diff !== null}
				<span class="diff" class:up={diff > 0} class:down={diff < 0} data-testid="rating-diff"
					>{formatDiff(diff)}</span
				>
			{/if}
		</span>
		<span class="side">{sideName(side)}</span>
	</span>
	<span class="time">{formatClock(ms)}</span>
</div>

<style>
	.clock {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		opacity: 0.7;
	}

	.clock.active {
		opacity: 1;
		border-color: var(--accent);
	}

	.clock.active.low {
		border-color: #e06c75;
	}

	.who {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}

	.name-line {
		display: flex;
		align-items: baseline;
		gap: 0.4rem;
		min-width: 0;
	}

	.name {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text);
		text-decoration: none;
	}

	a.name:hover {
		text-decoration: underline;
	}

	.rating {
		color: var(--text-muted);
		font-size: 0.85rem;
		font-variant-numeric: tabular-nums;
	}

	.diff {
		font-size: 0.85rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
	}

	.diff.up {
		color: var(--accent);
	}

	.diff.down {
		color: #e06c75;
	}

	.name.open {
		font-weight: 400;
		font-style: italic;
		color: var(--text-muted);
	}

	.side {
		color: var(--text-muted);
		font-size: 0.75rem;
	}

	.time {
		font-size: 1.4rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
	}

	.low .time {
		color: #e06c75;
	}
</style>
