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
import { join } from 'path';
import { onLogout } from '@/vue-apollo';
const { Menu, MenuItem, Tray } = remote;

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
		}
	},
	watch: {
		loggedIn() {
			this.menu = this.buildMenu();
			if (this.tray) this.tray.setContextMenu(this.buildMenu());
			else this.tray = new Tray(join(__static, 'logo.png'));
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

		if (!this.tray) this.tray = new Tray(join(__static, 'logo.png'));
		this.tray.setContextMenu(this.buildMenu());
	},
	beforeDestroy() {
		if (this.tray) this.tray.destroy();
	},
	methods: {
		buildMenu() {
			const menu = new Menu();
			menu.append(new MenuItem(
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
			menu.append(new MenuItem(
				{
					label: 'Settings', click() {
						ipcRenderer.send('settingsModal');
					}
				}
			));
			menu.append(new MenuItem({ type: 'separator' }));
			menu.append(new MenuItem(
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

			return menu;
		}
	}
};
</script>
