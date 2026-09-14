<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import MoveList from '$lib/components/MoveList.svelte';
	import { session } from '$lib/auth/session.svelte';
	import { awayText, gameEndText, sideName, statusText } from '$lib/chess/status';
	import type { OnlineGame } from '$lib/online/client.svelte';
	import { inviteLink } from '$lib/online/invites';
	import { withNext } from '$lib/auth/next';

	// Everything beside the board in an online game: status, the invite while
	// the Black seat is open, joining, draw offers, the move list and controls.
	interface Props {
		id: string;
		online: OnlineGame;
	}
	let { id, online }: Props = $props();
	const game = $derived(online.game);

	// White sees the invite until someone takes the Black seat.
	const invite = $derived(
		online.yourColor === 'white' && online.players.black === null ? inviteLink(id) : null
	);
	const atStart = $derived(game.view.cursor === 0);
	const atEnd = $derived(!game.view.viewingHistory);
	const loginHref = $derived(withNext(resolve('/login'), page.url.pathname));

	const connectionText: Record<typeof online.connection, string> = {
		connecting: 'Connecting…',
		open: '',
		reconnecting: 'Connection lost, reconnecting…',
		closed: 'Disconnected',
		failed: 'Could not connect to the server'
	};

	let joinError: string | null = $state(null);
	async function join() {
		joinError = null;
		try {
			await session.ensure();
			await online.join();
		} catch (e) {
			joinError = e instanceof Error ? e.message : String(e);
		}
	}

	let copied = $state(false);
	async function copyInvite() {
		if (!invite) return;
		await navigator.clipboard.writeText(invite);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<aside class="sidebar">
	{#if connectionText[online.connection]}
		<p class="status connection" role="status">{connectionText[online.connection]}</p>
	{/if}
	<p class="status" data-testid="game-status">
		{#if online.ended}
			{gameEndText(online.ended)}
		{:else if online.yourColor && online.players.black === null}
			Waiting for an opponent to join
		{:else if online.yourColor}
			{online.isMyTurn ? 'Your move' : 'Waiting for your opponent'}
			{#if game.view.status === 'check'}
				— check{/if}
		{:else}
			{statusText(game.view)}
		{/if}
	</p>
	{#if online.away}
		<p class="status away" role="status" data-testid="away">
			{awayText(online.away, online.yourColor, game.view.plyCount)}
		</p>
	{/if}
	{#if online.rejection}
		<p class="rejection" role="alert">{online.rejection}</p>
	{/if}
	{#if invite}
		<div class="invite">
			<p>Send this link to your opponent. They play Black.</p>
			<input
				readonly
				value={invite}
				data-testid="invite-link"
				onfocus={(e) => e.currentTarget.select()}
			/>
			<button type="button" onclick={copyInvite}>{copied ? 'Copied' : 'Copy link'}</button>
		</div>
	{/if}
	{#if online.canJoin}
		<div class="invite">
			<p>The Black seat is open.</p>
			<button type="button" onclick={join}>Join as Black</button>
			{#if !session.registered}
				<p class="hint">
					You would play as a guest. <a href={loginHref}>Log in</a> to keep the game on your account.
				</p>
			{/if}
			{#if joinError}
				<p class="rejection" role="alert">{joinError}</p>
			{/if}
		</div>
	{/if}
	{#if online.drawOffer && !online.ended}
		<div class="draw" role="status">
			{#if online.drawOffer === online.yourColor}
				<span>Draw offered. Waiting for a reply.</span>
			{:else if online.yourColor}
				<span>{sideName(online.drawOffer)} offers a draw.</span>
				<button type="button" onclick={() => online.acceptDraw()}>Accept</button>
				<button type="button" onclick={() => online.declineDraw()}>Decline</button>
			{:else}
				<span>{sideName(online.drawOffer)} offers a draw.</span>
			{/if}
		</div>
	{/if}
	<MoveList {game} />
	<div class="controls">
		<button type="button" title="First move" disabled={atStart} onclick={() => game.goToStart()}
			>⏮</button
		>
		<button type="button" title="Previous move" disabled={atStart} onclick={() => game.goBack()}
			>◀</button
		>
		<button type="button" title="Next move" disabled={atEnd} onclick={() => game.goForward()}
			>▶</button
		>
		<button type="button" title="Last move" disabled={atEnd} onclick={() => game.goToEnd()}
			>⏭</button
		>
	</div>
	{#if online.yourColor && !online.ended}
		<div class="controls">
			<button type="button" onclick={() => online.offerDraw()} disabled={online.drawOffer !== null}>
				Offer draw
			</button>
			<button type="button" class="danger" onclick={() => online.resign()}>Resign</button>
		</div>
	{/if}
</aside>

<style>
	.connection {
		color: var(--text-muted);
		font-weight: 400;
	}

	.away {
		border-color: var(--warning);
		font-weight: 400;
		font-variant-numeric: tabular-nums;
	}

	.rejection {
		margin: 0;
		color: #e06c75;
		font-size: 0.85rem;
	}

	.invite {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.6rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--accent);
		border-radius: 6px;
		font-size: 0.85rem;
	}

	.invite p {
		margin: 0;
	}

	.invite input {
		width: 100%;
		padding: 0.4rem 0.5rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--bg);
		color: var(--text);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.8rem;
	}

	.hint {
		color: var(--text-muted);
	}

	.hint a {
		color: var(--text);
	}

	.draw {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
		padding: 0.5rem 0.8rem;
		background: var(--panel);
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		font-size: 0.9rem;
	}

	.controls {
		display: flex;
		gap: 0.5rem;
	}

	button {
		min-height: var(--tap);
		flex: 1;
		padding: 0.45rem 0.6rem;
		border: 1px solid var(--panel-border);
		border-radius: 6px;
		background: var(--panel);
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	.draw button {
		flex: 0 0 auto;
	}

	button:hover:not(:disabled) {
		border-color: var(--text-muted);
	}

	button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	button.danger:hover {
		border-color: #e06c75;
		color: #e06c75;
	}
</style>
