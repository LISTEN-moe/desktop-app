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
	<div id="app"
		:class="radioType">
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
import WebSocketWorker from 'workerize-loader!@/assets/worker/websocket.worker.js';

export default {
	data() {
		return {
			tray: null,
			worker: null
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
	watch: {
		radioType() {
			if (!this.worker) return;
			this.worker.postMessage(this.radioType);
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
		if (localStorage.radioType) this.$store.dispatch('setRadioType', localStorage.radioType);
		this.$store.dispatch('setInitialState');

		this.worker = new WebSocketWorker();
		this.worker.onmessage = message => {
			if (message.data.method) return;

			/*
				TODO:
				Need to rewrite all of this since the api and the websocket return different
				format for the character data. If anyone clicks on `previous songs`, the parseData
				function that parses the incoming data doesn't work with this format.

				A problem for tomorrow
			*/
			if (message.data.song.characters && message.data.song.characters.length) {
				message.data.song.artists.forEach(artist => {
					if (!artist.characters || !artist.characters.length) return;
					const characters = [];
					artist.characters.forEach(character => {
						const found = message.data.song.characters.find(el => el.id === character.id);
						if (found) characters.push(found);
					});
					artist.characters = characters;
				});
			}

			// Delete character data from lastPlayed so the table displays correctly
			message.data.lastPlayed.forEach(song => {
				song.artists.forEach(artist => {
					delete artist.characters;
				});
				delete song.characters;
			});

			this.$store.commit('websocket', message.data);
		};

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
