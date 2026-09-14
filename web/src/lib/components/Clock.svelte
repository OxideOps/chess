<script lang="ts">
	import { sideName } from '$lib/chess/status';
	import { formatClock } from '$lib/online/clock';
	import type { Side } from '$lib/generated/Side';

	// One side's remaining time and who is playing it. `active` while that
	// side's clock is running.
	interface Props {
		ms: number;
		side: Side;
		/** The player's display name; `null` while the seat is open. */
		name?: string | null;
		active?: boolean;
	}
	let { ms, side, name = null, active = false }: Props = $props();
	const low = $derived(ms < 20_000);
</script>

<div class="clock" class:active class:low aria-label="{sideName(side)} clock">
	<span class="who">
		<span class="name" class:open={name === null}>{name ?? 'Open seat'}</span>
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

	.name {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
