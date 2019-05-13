<style lang="scss">
	@import "@/assets/styles/_colors.scss";

	h1.title {
		font-size: 1.2rem;
		color: $textColor;
		margin-bottom: 1rem;
	}

	.card {
		-webkit-app-region: drag;
		overflow: hidden !important;
		min-height: 100%;

		.card-body {
			overflow: hidden !important;
			max-height: unset !important;

			button { 
				-webkit-app-region: no-drag;
				width: 100%
			}

			.buttonContainer {
				margin-bottom: 5px;
				margin-right: 5px;
				min-width: 80%;
			}

			.closeContainer {
				display: block;
				position: fixed;
				top: 10px;
				right: 10px;
			}

			.loggedInButton {
				width: 100%;

				button { width: 80%; }

			}

			a {
				color: $basePink;
				display: block;
				margin-bottom: 10px;
			}
		}
	}

</style>

<template>
	<div class="modal">
		<div class="card dark shadow">
			<div class="card-body">
				<Close @clicked="closeWindow" />
				<img src="@/assets/images/logo-square-64.png">

				<h1 class="title">General settings</h1>
				<Toggle :active="preferRomaji"
					@clicked="setOption('preferRomaji')">Prefer romaji over japanese</Toggle>
				<Toggle :active="eventStarts"
					@clicked="setOption('eventStarts')">Desktop notification when an event starts</Toggle>
				<Toggle :active="alwaysOnTop"
					@clicked="setOption('alwaysOnTop')">Always on top</Toggle>
				
				<h1 class="title">Tray settings</h1>
				<Toggle :active="hideFromTaskbar"
					@clicked="setOption('hideFromTaskbar')">Hide from taskbar</Toggle>
				<!-- <Toggle :active="minimizeToTray"
				 	@clicked="setOption('minimizeToTray')">Minimize to tray</Toggle> -->

				<h1 class="title">Theme & Layout</h1>
				<Toggle :active="enableGaps"
					@clicked="setOption('enableGaps')">Enable gaps</Toggle>
				<Toggle :active="smallAlbumArt"
					@clicked="setOption('smallAlbumArt')">Small album art</Toggle>

				<h1 class="title">Account</h1>

				<a v-if="loggedIn"
					:href="`https://beta.listen.moe/u/${user.username}`"
					target="_blank">
					Logged in as {{ user.displayName }}
				</a>
				<div class="loggedInButton">
					<button v-if="!loggedIn"
						class="primary light"
						@click.stop.prevent="login">Login</button>
					<button v-else
						class="primary light"
						@click.stop.prevent="logout">Logout</button>
				</div>
			</div>
		</div>
	</div>
</template>

<script>
import Toggle from '@/components/toggle';
import Close from '@/components/buttons/close';
import { remote, ipcRenderer } from 'electron';
import { onLogout } from '@/vue-apollo';

export default {
	components: {
		Toggle,
		Close
	},
	computed: {
		preferRomaji() {
			return this.$store.state.preferRomaji;
		},
		eventStarts() {
			return this.$store.state.eventStarts;
		},
		enableGaps() {
			return this.$store.state.enableGaps;
		},
		smallAlbumArt() {
			return this.$store.state.smallAlbumArt;
		},
		loggedIn() {
			return this.$store.state.loggedIn;
		},
		user() {
			return this.$store.state.user;
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
	mounted() {
		ipcRenderer.on('login', (_, { token, user }) => {
			this.$store.dispatch('login', { token, user });
		});
	},
	methods: {
		login() {
			ipcRenderer.send('loginModal');
		},
		async logout() {
			this.$store.dispatch('logout');
			await onLogout(this.$apollo);
		},
		setOption(option) {
			const currentValue = this.$store.state[option];
			this.$store.dispatch('setState', { option, value: !currentValue });
			ipcRenderer.send('settingsChange', [option, !currentValue]);
		},
		closeWindow() {
			return remote.getCurrentWindow().close();
		}
	}
};
</script>
