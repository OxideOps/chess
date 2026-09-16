<script lang="ts">
	import { barFraction, formatScore } from '$lib/chess/wasm';
	import type { EngineScore } from '$lib/generated/EngineScore';
	import type { Side } from '$lib/generated/Side';

	// A vertical bar showing who is better. `score` is from White's point of
	// view; `null` shows an even bar with no label.
	let { score, orientation = 'white' }: { score: EngineScore | null; orientation?: Side } =
		$props();

	// Never let either side vanish completely, so the bar still reads as a bar.
	const white = $derived(score ? Math.min(0.96, Math.max(0.04, barFraction(score))) : 0.5);
	const label = $derived(
		score
			? score.kind === 'cp'
				? (Math.abs(score.value) / 100).toFixed(1)
				: `#${Math.abs(score.value)}`
			: null
	);
	const whiteAhead = $derived(white >= 0.5);
	const title = $derived(
		score ? `Evaluation ${formatScore(score)} (White's point of view)` : 'No evaluation'
	);
</script>

<div class="eval-bar" class:flipped={orientation === 'black'} {title} data-testid="eval-bar">
	<div class="white" style:height="{white * 100}%"></div>
	{#if label}
		<span class="label" class:for-white={whiteAhead} class:for-black={!whiteAhead}>{label}</span>
	{/if}
</div>

<style>
	.eval-bar {
		position: relative;
		width: 1.4rem;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		background: var(--piece-black);
		border-radius: 4px;
		overflow: hidden;
		box-shadow: var(--shadow);
	}

	.eval-bar.flipped {
		justify-content: flex-start;
	}

	.white {
		width: 100%;
		background: var(--piece-white);
		transition: height 0.4s ease;
	}

	.label {
		position: absolute;
		left: 0;
		right: 0;
		text-align: center;
		font-family: var(--font-mono);
		font-size: 0.6rem;
		font-weight: 700;
		font-variant-numeric: tabular-nums;
		line-height: 1;
		padding: 4px 0;
	}

	/* The label sits at the winning side's end of the bar. */
	.label.for-white {
		bottom: 0;
		color: var(--piece-black);
	}

	.label.for-black {
		top: 0;
		color: var(--piece-white);
	}

	.flipped .label.for-white {
		top: 0;
		bottom: auto;
	}

	.flipped .label.for-black {
		bottom: 0;
		top: auto;
	}
</style>
