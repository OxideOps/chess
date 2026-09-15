<script lang="ts">
	import type { Explanation } from '$lib/generated/Explanation';

	// A coach answer, with the moves it names from the engine's lines marked
	// (the server marks them: `AnswerPart`). Hovering, focusing or tapping a
	// move shows it as an arrow; with `onplay`, clicking plays its line.
	export interface Arrow {
		from: string;
		to: string;
	}

	interface Props {
		answer: Explanation;
		onpreview?: (arrow: Arrow | null) => void;
		/** Play the moves from the explained position to this one (UCI). */
		onplay?: (path: string[]) => void;
		testid?: string;
	}
	let { answer, onpreview, onplay, testid }: Props = $props();

	const arrow = (path: string[]): Arrow => {
		const last = path[path.length - 1];
		return { from: last.slice(0, 2), to: last.slice(2, 4) };
	};
</script>

<!-- No whitespace between the parts: it would show as spaces around the moves. -->
<p class="text" data-testid={testid}>
	{#each answer.parts as part, i (i)}{#if part.kind === 'text'}{part.text}{:else}<button
				type="button"
				class="move-ref"
				title={onplay ? `Play ${part.text} on the board` : `Show ${part.text} on the board`}
				onmouseenter={() => onpreview?.(arrow(part.path))}
				onmouseleave={() => onpreview?.(null)}
				onfocus={() => onpreview?.(arrow(part.path))}
				onblur={() => onpreview?.(null)}
				onclick={() => {
					if (onplay) {
						onpreview?.(null);
						onplay(part.path);
					} else {
						onpreview?.(arrow(part.path));
					}
				}}>{part.text}</button
			>{/if}{/each}
</p>

<style>
	.text {
		margin: 0;
		line-height: 1.45;
		white-space: pre-line;
	}

	.move-ref {
		display: inline;
		padding: 0 0.1em;
		border: none;
		border-bottom: 1px dotted var(--accent);
		border-radius: 3px;
		background: none;
		color: inherit;
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}

	.move-ref:hover,
	.move-ref:focus-visible {
		background: var(--highlight-last-move);
		outline: none;
	}
</style>
