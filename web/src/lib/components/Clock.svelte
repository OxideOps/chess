<script lang="ts">
	import { formatClock } from '$lib/online/clock';

	// One side's remaining time. `active` while that side's clock is running.
	let { ms, active = false, label }: { ms: number; active?: boolean; label: string } = $props();
	const low = $derived(ms < 20_000);
</script>

<div class="clock" class:active class:low aria-label="{label} clock">
	<span class="label">{label}</span>
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

	.label {
		color: var(--text-muted);
		font-size: 0.85rem;
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
