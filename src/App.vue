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

	input, div, span, a {
		outline: none !important;
	}

	div, span, h1, h2, a, label {
		font-size: 1rem;
		font-style: normal;
		font-weight: 400;
		text-rendering: optimizeLegibility;
		text-size-adjust: 100%;
		-webkit-font-smoothing: antialiased;
		// -webkit-user-select: none;
	}
</style>

<template>
	<div id="app"
		ref="app"
		@mousedown.stop.prevent="mousedown"
		@mouseup.stop.prevent="mouseup"
		@dragstart.stop.prevent="false"
		@mousemove.stop.prevent="drag">
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
import { onLogout } from '@/vue-apollo';
const { Menu, MenuItem } = remote;

export default {
	data() {
		return {
			menu: null,
			mouseDown: false,
			offset: null
		};
	},
	computed: {
		loggedIn() {
			return this.$store.state.loggedIn;
		},
		radioType() {
			return this.$store.state.radioType;
		}
	},
	watch: {
		loggedIn() {
			this.buildMenu();
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
					this.$store.commit('user', data.user);
					this.$store.commit('loggedIn', true);
					this.$store.commit('token', token);
					this.$forceUpdate();
				}
			} catch {}
		}
		this.$store.dispatch('setInitialState');

		ipcRenderer.on('login', (_, { token, user }) => {
			this.$store.dispatch('login', { token, user });
		});
		ipcRenderer.on('playerOptionsChange', (_, arg) => {
			const [option, value] = arg;
			const electronWindow = remote.getCurrentWindow();
			if (option === 'smallAlbumArt' && value) electronWindow.setSize(electronWindow.getBounds().width, 80, true);
			else if (option === 'smallAlbumArt' && !value) electronWindow.setSize(electronWindow.getBounds().width, 230, true);
			this.$store.dispatch('setState', { option, value });
		});

		this.buildMenu();

		window.addEventListener('contextmenu', e => {
			e.preventDefault();
			this.menu.popup({ window: remote.getCurrentWindow() });
		}, false);
	},
	methods: {
		buildMenu() {
			this.menu = new Menu();
			this.menu.append(new MenuItem(
				{
					label: 'Switch to kpop',
					type: 'checkbox',
					checked: this.isJpop ? false : true,
					click: () => {
						if (this.radioType === 'kpop') this.$store.commit('radioType', 'jpop');
						else this.$store.commit('radioType', 'kpop');
						this.buildMenu();
					}
				}
			));
			this.menu.append(new MenuItem(
				{
					label: 'Settings', click() {
						ipcRenderer.send('settingsModal');
					}
				}
			));
			this.menu.append(new MenuItem({ type: 'separator' }));
			this.menu.append(new MenuItem(
				{
					label: this.loggedIn ? 'Logout' : 'Login',
					click: async () => {
						if (this.loggedIn) {
							this.$store.dispatch('logout');
							await onLogout(this.$apollo);
						} else {
							ipcRenderer.send('loginModal');
						}
						this.buildMenu();
					}
				}
			));
		},
		mousedown({ clientX, clientY }) {
			this.offset = [clientX, clientY];
			this.mouseDown = true;
		},
		mouseup() {
			this.offset = null;
			this.mousDown = false;
		},
		drag({ screenX, screenY }) {
			if (!this.offset || !this.offset.length) return;

			const x = Math.round(screenX - this.offset[0]);
			const y = Math.round(screenY - this.offset[1]);
			remote.getCurrentWindow().setPosition(x, y);
		}
	}
};
</script>
