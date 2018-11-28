<style lang="scss">
	@import url('https://fonts.googleapis.com/css?family=Nunito:400,700');

	* {
		box-sizing: border-box;
	}

	body {
		height: 100vh;
		overflow: hidden;
		display: flex;
		flex-grow: 1;
	}

	#app {
		font-family: 'Nunito', BlinkMacSystemFont, -apple-system, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", "Helvetica", "Arial", sans-serif;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		text-align: center;
		color: #2c3e50;

		display: flex;
		flex-grow: 1;
	}
</style>

<template>
	<div id="app">
		<audio id="audio-player"
			ref="audio"
			crossorigin="anonymous"
			preload="auto" />
		<Player :audio="$refs" />
	</div>
</template>

<script>
import WebSocketWorker from 'workerize-loader!@/assets/worker/websocket.worker.js';
import Player from '@/components/player';

export default {
	components: { Player },
	mounted() {
		const worker = new WebSocketWorker();
		worker.onmessage = message => {
			if (message.data.method) return;
			this.$store.commit('websocket', message.data);
		};
	}
};
</script>
