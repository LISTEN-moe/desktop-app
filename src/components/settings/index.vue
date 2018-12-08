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

		.card-body {
			overflow: hidden !important;

			button { -webkit-app-region: no-drag; }
			.buttonContainer {
				margin-bottom: 5px;
				margin-right: 5px;
			}

			.closeButton {
				display: block;
				margin: 0 auto;
				margin-top: 2rem;
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
				<button v-if="!loggedIn"
					class="primary light"
					@click.stop.prevent="login">Login</button>
				<button v-else
					class="primary light"
					@click.stop.prevent="logout">Logout</button>
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
		}
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
