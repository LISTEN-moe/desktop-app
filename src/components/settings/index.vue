<style lang="scss">
	@import "@/assets/styles/_colors.scss";

	h1.title {
		font-size: 1.2rem;
		color: $textColor;
		margin-bottom: 1rem;
	}

	.card .card-body {
		overflow: hidden !important;

		.buttonContainer {
			margin-bottom: 5px;
			margin-right: 5px;
		}

		.closeButton {
			display: block;
			margin: 0 auto;
			margin-top: 2rem;
		}
	}
</style>

<template>
	<div class="modal">
		<div class="card dark shadow">
			<div class="card-body">
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

				<button class="primary light closeButton"
					@click="closeWindow">Close</button>
			</div>
		</div>
	</div>
</template>

<script>
import Toggle from '@/components/toggle';
import { remote, ipcRenderer } from 'electron';

export default {
	components: {
		Toggle
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
		}
	},
	methods: {
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
