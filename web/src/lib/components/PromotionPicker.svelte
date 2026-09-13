<script lang="ts">
	import { pieceImage } from './pieces';
	import type { Promotion } from '$lib/chess/game.svelte';
	import type { Side } from '$lib/generated/Side';

	// Overlay asking which piece a pawn should become. Calls `onpick(null)` when dismissed.
	let { color, onpick }: { color: Side; onpick: (role: Promotion | null) => void } = $props();

	const roles: Promotion[] = ['queen', 'rook', 'bishop', 'knight'];
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="promotion" onclick={() => onpick(null)}>
	{#each roles as role (role)}
		<button
			type="button"
			title={role}
			onclick={(event) => {
				event.stopPropagation();
				onpick(role);
			}}
		>
			<img src={pieceImage(color, role)} alt={role} />
		</button>
	{/each}
</div>

<style>
	.promotion {
		position: absolute;
		z-index: 10;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		background: rgba(0, 0, 0, 0.55);
	}

	button {
		width: 17%;
		aspect-ratio: 1;
		padding: 4%;
		border: none;
		border-radius: 12px;
		background: var(--square-light);
		cursor: pointer;
	}

	button:hover {
		background: #fff;
	}

	img {
		width: 100%;
		height: 100%;
	}
</style>
