<style lang="scss">
	@import url('https://fonts.googleapis.com/css?family=Nunito:400,700');
	@import "@/assets/styles/components.scss";

	* {
		box-sizing: border-box;
	}

	body {
		overflow: hidden;
		margin: 0;
	}

	#app {
		font-family: 'Nunito', BlinkMacSystemFont, -apple-system, "Segoe UI", "Roboto", "Oxygen", "Ubuntu", "Cantarell", "Fira Sans", "Droid Sans", "Helvetica Neue", "Helvetica", "Arial", sans-serif;
		-webkit-font-smoothing: antialiased;
		-moz-osx-font-smoothing: grayscale;
		text-align: center;
		color: #2c3e50;
	}

	div, span, h1, h2, a, label {
		font-size: 1rem;
		font-style: normal;
		font-weight: 400;
		text-rendering: optimizeLegibility;
		text-size-adjust: 100%;
		-webkit-font-smoothing: antialiased;
	}

	input, div, span, a {
		outline: none !important;
	}
</style>

<template>
	<div id="app">
		<audio id="audio-player"
			ref="audio"
			crossorigin="anonymous"
			preload="auto" />
		<router-view :audio="$refs" />
	</div>
</template>

<script>
import user from '@/gql/queries/user.gql';
import { ipcRenderer, remote } from 'electron';

export default {
	data() {
		return {
			tray: null
		};
	},
	computed: {
		loggedIn() {
			return this.$store.state.loggedIn;
		},
		radioType() {
			return this.$store.state.radioType;
		},
		alwaysOnTop() {
			return this.$store.state.alwaysOnTop;
		},
		hideFromTaskbar() {
			return this.$store.state.hideFromTaskbar;
		},
		minimizeToTray() {
			return this.$store.state.minimizeToTray;
		}
	},
	async mounted() {
		const token = localStorage.getItem('token');
		if (token) {
			try {
				const { data } = await this.$apollo.query({
					query: user,
					variables: {
						username: '@me'
					}
				});

				if (data.user) {
					this.$store.dispatch('login', { token, user: data.user });
					this.$forceUpdate();
				}
			} catch {}
		}
		this.$store.dispatch('setInitialState');

		ipcRenderer.send('settingsChange', ['alwaysOnTop', this.alwaysOnTop]);
		ipcRenderer.send('settingsChange', ['hideFromTaskbar', this.hideFromTaskbar]);
		ipcRenderer.send('settingsChange', ['minimizeToTray', this.minimizeToTray]);
		ipcRenderer.on('login', (_, { token, user }) => {
			this.$store.dispatch('login', { token, user });
		});
		ipcRenderer.on('playerOptionsChange', (_, arg) => {
			const [option, value] = arg;
			const electronWindow = remote.getCurrentWindow();
			if (option === 'smallAlbumArt' && value) electronWindow.setSize(electronWindow.getBounds().width, 80, true);
			else if (option === 'smallAlbumArt' && !value) electronWindow.setSize(electronWindow.getBounds().width, 230, true);
			else if (option === 'alwaysOnTop') electronWindow.setAlwaysOnTop(value);
			else if (option === 'hideFromTaskbar') electronWindow.setSkipTaskbar(value);
			this.$store.dispatch('setState', { option, value });
		});
	}
};
</script>
