<style lang="scss">
</style>

<template>
	<Player :audio="audio" />
</template>

<script>
import WebSocketWorker from 'workerize-loader!@/assets/worker/websocket.worker.js';
import Player from '@/components/player';

export default {
	components: { Player },
	props: {
		audio: {
			'type': Object,
			'default': () => {}
		}
	},
	mounted() {
		const worker = new WebSocketWorker();
		worker.onmessage = message => {
			if (message.data.method) return;
			this.$store.commit('websocket', message.data);
		};
	}
};
</script>
