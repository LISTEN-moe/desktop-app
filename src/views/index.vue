<style lang="scss">
</style>

<template>
	<Player :audio="audio" />
</template>

<script>
import Player from '@/components/player';

export default {
	components: { Player },
	props: {
		audio: {
			'type': Object,
			'default': () => {}
		}
	}
};
</script>
