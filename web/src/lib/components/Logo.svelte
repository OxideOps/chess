<script lang="ts">
	// The site's mark: a rook drawn in the board's own right angles — no
	// curves, every edge on the square grid the game is played on. Takes
	// `currentColor`, so it sits on any surface.
	interface Props {
		/** Rendered size in pixels. */
		size?: number;
	}
	let { size = 22 }: Props = $props();
</script>

<svg
	class="logo"
	width={size}
	height={size}
	viewBox="88 88 336 336"
	fill="currentColor"
	aria-hidden="true"
	focusable="false"
>
	<rect x="116" y="100" width="64" height="56" />
	<rect x="224" y="100" width="64" height="56" />
	<rect x="332" y="100" width="64" height="56" />
	<rect x="116" y="156" width="280" height="56" />
	<rect x="144" y="212" width="224" height="28" />
	<rect x="172" y="240" width="168" height="84" />
	<rect x="144" y="324" width="224" height="28" />
	<rect x="88" y="352" width="336" height="60" />
</svg>

<style>
	.logo {
		display: block;
		flex: none;
	}
</style>
