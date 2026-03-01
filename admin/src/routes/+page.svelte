<script lang="ts">
	import { goto } from '$app/navigation';
	import { adminAuthed } from '$lib/stores/auth.js';
	import { onMount } from 'svelte';

	onMount(() => {
		if ($adminAuthed) {
			goto('/overview');
		}
	});

	$effect(() => {
		if ($adminAuthed) {
			goto('/overview');
		}
	});
</script>

<!-- Login is handled by the layout; this page just redirects when authed -->
