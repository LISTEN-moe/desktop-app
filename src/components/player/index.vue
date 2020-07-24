<style lang="scss" scoped>
	@import "@/assets/styles/_colors.scss";

	div.playerContainer {
		display: grid;
		grid-template-columns: 64px 1fr 64px 64px auto;
		grid-template-rows: 64px;
		grid-template-areas: "playButton player favoriteButton volumeButton coverArt";
		width: 100vw;
		-webkit-app-region: drag;

		a, span, svg, img {
			-webkit-app-region: no-drag;
		}

		&.hasAlbumArt {
			&.big {
				grid-template-rows: 64px 64px 64px;
				grid-template-areas:
					". . . . coverArt"
					"playButton player favoriteButton volumeButton coverArt"
					". . . . coverArt";
			}
		}

		&.gaps {
			grid-gap: 5px;
		}

		&:not(.gaps) > .shadow { box-shadow: none; }

		.playButton {
			grid-area: playButton;
		}

		.player {
			grid-area: player;
			position: relative;
			display: grid;
			grid-template-columns: 100%;
			grid-template-rows: auto auto;
			overflow: hidden;

			// Span and links color
			color: #c7ccd8;
			a {
				color: $basePink;
				text-decoration: none;
				-webkit-app-region: no-drag;

				&.source {
					color: $textColor;
					text-decoration: underline;
				}
			}

			> div {
				padding: 0px 10px;
				z-index: 1;
			}

			.artistContainer {
				padding-top: 10px;
				text-align: left;
			}

			.titleContainer {
				padding-bottom: 10px;
				text-align: left;
			}

			.info {
				max-height: 22px;
				background: linear-gradient(to right, rgba(0,0,0,0) 0%,rgba(0,0,0,0.2) 30%,rgba(0, 0, 0, 0.2) 70%,rgba(0,0,0,0) 100%);

				.requestedBy, .eventTime {
					font-size: .8rem;

					& span, a {
						font-size: .8rem;
					}

					& a {
						font-weight: 700;
					}
				}
			}

			canvas {
				position: absolute;
				pointer-events: none;
				width: 100%;
				left: 0;
				bottom: 0;
				height: 64px;
				z-index: 0;
			}

			.backdrop {
				position: absolute;
				top: 0;
				left: 0;
				width: 100%;
				height: 100%;
				z-index: 10;
				background: black;
				opacity: .35;
				display: none;

				&.active {
					display: block;
				}
			}

			.sliderContainer {
				position: absolute;
				top: 20px;
				right: 20px;
				width: 100%;
				z-index: 11;
				max-width: 200px;
				opacity: 0;
				pointer-events: none;
				-webkit-app-region: no-drag;

				> * { -webkit-app-region: no-drag; }

				&.active {
					opacity: 1;
					pointer-events: auto;
				}
			}

			&.hasRequester {
				grid-template-rows: auto auto auto;

				.artistContainer {
					padding-top: 5px;
				}

				.titleContainer {
					padding-bottom: 0px;
				}
			}

			.settingsIcon {
				position: absolute;
				top: 0;
				right: 0;
				margin: 0;
				transform: scale3d(0.5, 0.5, 1) !important;
				z-index: 9;
				transition: opacity .3s ease-in-out;
				opacity: .75;
			}

			.hideIcon {
				position: absolute;
				top: 0;
				right: 1.2rem;
				margin: 0;
				transform: scale3d(0.5, 0.5, 1) !important;
				z-index: 9;
				transition: opacity .3s ease-in-out;
				opacity: .75;
			}

		}

		.favoriteButton {
			grid-area: favoriteButton;
		}

		.volumeButton {
			grid-area: volumeButton;
		}

		.albumContainer {
			grid-area: coverArt;
			max-width: 64px;
			background: none;

			a {
				display: block;
				max-height: 64px;

				img {
					max-width: 64px;
				}
			}

			&.big {
				max-width: 250px;
				div.pictureContainer {
					img {
						width: 200px;
						max-width: 200px;
						max-height: 250px;
					}
				}
			}
		}

		> div {
			background: #1d1f2b;

			> svg {
				margin-top: 20px;
				width: 24px;
				height: 24px;
			}
		}
	}
	#app.kpop .playerContainer .radio-switch button:hover {
		color: $baseBlue;
	}

	#app.kpop .playerContainer .player {
		.info {
			a {
				color: $baseBlue;
			}

			.eventTime span {
				color: $baseBlue;
			}
		}

		.artistContainer {
			a {
				color: $baseBlue;
			}
		}
	}
</style>

<template>
	<div class="playerContainer"
		:class="{ gaps: enableGaps, big: !smallAlbumArt, hasAlbumArt: currentAlbum && albumCover }">
		<div class="playButton">
			<Budicon :icon="playing ? 'pause' : 'play'"
				y="-2"
				@click.native.stop.prevent="togglePlaying" />
		</div>
		<div class="player"
			:class="{ hasRequester: currentRequester || currentEvent }">
			<div class="backdrop"
				:class="{ active: isVolumeSliderOpen }" />
			<Budicon class="settingsIcon"
				icon="settings"
				@click.native.stop.prevent="openSettings" />
			<Budicon v-if="minimizeToTray"
				class="hideIcon"
				icon="cross"
				@click.native.stop.prevent="hideWindow" />
			<canvas ref="canvas" />
			<div class="sliderContainer"
				:class="{ active: isVolumeSliderOpen }">
				<Slider
					ref="slider"
					v-model="volume"
					tooltip="hover"
					tooltipDir="bottom"
					:sliderStyle="{ background: radioType === 'jpop' ? '#c40447' : '#1587c9'}"
					:processStyle="{ background: radioType === 'jpop' ? '#ff015b' : '#30A9ED'}"
					:tooltipStyle="{ backgroundColor: '#1d1f2b', borderColor: '#1d1f2b' }"
					@callback="setVolume" />
			</div>
			<Marquee v-if="currentArtists"
				ref="marquee"
				class="artistContainer">
				<span class="ja player-song-artist-container">
					<span v-for="(artist, index) in currentArtists"
						:key="artist.id"
						class="player-song-artist">
						<template v-if="artist.characters && artist.characters.length && artist.characters[0].name">
							<template v-for="(character, index2) in artist.characters">
								<a :key="character.id" :href="`https://listen.moe/characters/${character.id}`">
									{{ preferRomaji ? character.nameRomaji ? character.nameRomaji : character.name ? character.name : character.nameRomaji : character.name ? character.name : character.nameRomaji }}
								</a>

								<a :key="`artist${character.id}`" :href="`https://listen.moe/artists/${artist.id}`">
									(CV: {{ preferRomaji ? artist.nameRomaji ? artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji }})
								</a>
								<template v-if="index2 < artist.characters.length - 1">, </template>
							</template>
						</template>
						<a v-else :href="`https://listen.moe/artists/${artist.id}`">
							{{ preferRomaji ? artist.nameRomaji ? artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji }}
						</a>
						<template v-if="index < currentArtists.length - 1">, </template>
					</span>
				</span>
			</Marquee>

			<Marquee v-if="currentSong && currentSong.name"
				ref="marquee"
				class="titleContainer">
				<span class="ja player-song-title">
					{{ currentSong.name }}
					<a v-if="currentSource"
						:href="currentSource.link"
						class="source">[{{ currentSource.name }}]</a>
				</span>
			</Marquee>
			<div v-if="currentRequester || currentEvent"
				class="info">
				<div v-if="currentRequester"
					class="requestedBy text">
					Requested by <a :href="currentRequester.link">{{ currentRequester.name }}</a>
				</div>
				<div v-else-if="currentEvent"
					class="eventTime text">
					<span>♫♪.ılılıll {{ currentEvent }} llılılı.♫♪</span>
				</div>
			</div>
		</div>
		<div class="favoriteButton">
			<Budicon v-if="websocket"
				:icon="websocket.song.favorite ? 'filledStar' : 'star'"
				@click.native.stop.prevent="toggleFavorite" />
		</div>
		<div class="volumeButton"
			@wheel.stop.prevent="scrollVolume">
			<Budicon icon="volume"
				@click.native.stop.prevent="isVolumeSliderOpen = !isVolumeSliderOpen" />
		</div>

		<div v-if="currentAlbum"
			class="albumContainer"
			:class="{ big: !smallAlbumArt }">
			<div class="pictureContainer">
				<a :href="currentAlbum.link">
					<img v-if="albumCover"
						class="shadow"
						:src="`https://cdn.listen.moe/covers/${albumCover}`">
					<img v-else
						class="shadow"
						src="@/assets/images/blank-dark.png">
				</a>
			</div>
		</div>
	</div>
</template>

<script>
import favoriteSong from '@/gql/mutations/favoriteSong.gql';
import checkFavorite from '@/gql/queries/checkFavorite.gql';
import Budicon from '@/components/icons/budicon';
import Marquee from '@/components/marquee';
import Slider from '@/components/slider/slider';

import { ipcRenderer, remote } from 'electron';
import { join } from 'path';
import { onLogout } from '@/vue-apollo';
const { Menu, MenuItem, Tray, app } = remote;

const MEDIA_ELEMENT_NODES = new WeakMap();

const FFT_SIZE = 1024;
const MIN_DECIBELS = -80;
const MAX_DECIBELS = -30;

let AUDIO_CONTEXT;
let SOURCE;
let ANALYSER;

let BUFFER_LENGTH;
let FREQUENCY_DATA;

const BAR_COLOR = '#d01356';

let MUSIC_VISUALS;

export default {
	components: {
		Budicon,
		Marquee,
		Slider
	},
	props: {
		audio: {
			'type': Object,
			'default': () => {}
		}
	},
	data() {
		return {
			volume: 0,
			isVolumeSliderOpen: false,
			tray: null
		};
	},
	computed: {
		user() {
			return this.$store.state.user;
		},
		loggedIn() {
			return this.$store.state.loggedIn;
		},
		preferRomaji() {
			return this.$store.state.preferRomaji;
		},
		radioType() {
			return this.$store.state.radioType;
		},
		playing() {
			return this.$store.state.playing;
		},
		websocket() {
			return this.$store.state.websocket;
		},
		hideFromTaskbar() {
			return this.$store.state.hideFromTaskbar;
		},
		currentRequester() {
			if (this.websocket && this.websocket.requester) {
				return {
					name: this.websocket.requester.displayName,
					link: `https://listen.moe/u/${this.websocket.requester.username}`
				};
			}
			return null;
		},
		currentEvent() {
			if (this.websocket && this.websocket.event) return this.websocket.event.name;
			return null;
		},
		currentListeners() {
			if (this.websocket && this.websocket.listeners) return this.websocket.listeners;
			return null;
		},
		albumCover() {
			if (this.websocket && this.websocket.song && this.websocket.song.albums && this.websocket.song.albums.length && this.websocket.song.albums[0].image) return this.websocket.song.albums[0].image;
			return null;
		},
		currentArtists() {
			if (this.websocket && this.websocket.song && this.websocket.song.artists && this.websocket.song.artists.length) return this.websocket.song.artists;
			return null;
		},
		currentSong() {
			if (this.websocket && this.websocket.song && this.websocket.song.title) {
				const data = {};
				data.name = this.preferRomaji
					? this.websocket.song.titleRomaji
						? this.websocket.song.titleRomaji
						: this.websocket.song.title
							? this.websocket.song.title
							: this.websocket.song.titleRomaji
					: this.websocket.song.title
						? this.websocket.song.title
						: this.websocket.song.titleRomaji;
				return data;
			}
			return null;
		},
		currentSource() {
			if (this.websocket && this.websocket.song && this.websocket.song.sources && this.websocket.song.sources.length) {
				const source = {};
				source.name = this.preferRomaji
					? this.websocket.song.sources[0].nameRomaji
						? this.websocket.song.sources[0].nameRomaji
						: this.websocket.song.sources[0].name
							? this.websocket.song.sources[0].name
							: this.websocket.song.sources[0].nameRomaji
					: this.websocket.song.sources[0].name
						? this.websocket.song.sources[0].name
						: this.websocket.song.sources[0].nameRomaji;

				source.link = `https://listen.moe/sources/${this.websocket.song.sources[0].id}`;
				return source;
			}
			return null;
		},
		currentAlbum() {
			if (this.websocket && this.websocket.song && this.websocket.song.albums && this.websocket.song.albums.length) {
				const album = {};
				album.name = this.preferRomaji
					? this.websocket.song.albums[0].nameRomaji
						? this.websocket.song.albums[0].nameRomaji
						: this.websocket.song.albums[0].name
							? this.websocket.song.albums[0].name
							: this.websocket.song.albums[0].nameRomaji
					: this.websocket.song.albums[0].name
						? this.websocket.song.albums[0].name
						: this.websocket.song.albums[0].nameRomaji;

				album.link = `https://listen.moe/albums/${this.websocket.song.albums[0].id}`;
				return album;
			}
			return null;
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
		minimizeToTray() {
			return this.$store.state.minimizeToTray;
		}
	},
	watch: {
		async websocket() {
			if (this.loggedIn) await this.checkFavorite();
			if (this.playing) this.updateDiscordActivity();
			if (this.$refs && this.$refs.slider) this.$nextTick(() => this.$refs.slider.refresh());
			this.buildTray();
		},
		loggedIn() {
			this.buildTray();
		},
		radioType() {
			this.buildTray();
			if (this.playing) {
				this.togglePlaying();
				this.togglePlaying();
			}
		},
		playing() {
			this.buildTray();
		}
	},
	mounted() {
		this.volume = window.localStorage ? localStorage.getItem('volume') ? localStorage.getItem('volume') * 100 : 50 : 50;

		this.buildTray();

		this.tray.on('click', () => ipcRenderer.send('show-tray'));

		MUSIC_VISUALS = {
			start: () => {
				if (!this.$refs.canvas) return;
				const ctx = this.$refs.canvas.getContext('2d');
				this.$refs.canvas.width = window.innerWidth;
				ctx.clearRect(0, 0, this.$refs.canvas.width, this.$refs.canvas.height);
				ctx.fillStyle = BAR_COLOR;

				const width = Math.ceil(this.$refs.canvas.width / ((BUFFER_LENGTH / 3) - 1));
				for (let i = 0; i < BUFFER_LENGTH; i++) {
					const height = (Math.max(0, FREQUENCY_DATA[i] - 24) / (236 - 24)) * this.$refs.canvas.height * 0.8;
					ctx.fillRect(i * width, this.$refs.canvas.height - height, width, height);
				}

				ANALYSER.getByteFrequencyData(FREQUENCY_DATA);
				requestAnimationFrame(MUSIC_VISUALS.start);
			},
			stop: () => {
				cancelAnimationFrame(requestAnimationFrame(MUSIC_VISUALS.start));
			}
		};
		if (!this.audio.audio.paused) MUSIC_VISUALS.start();
	},
	beforeDestroy() {
		if (this.tray) this.tray.destroy();
	},
	methods: {
		openSettings() {
			ipcRenderer.send('settingsModal');
		},
		hideWindow() {
			ipcRenderer.send('hide-tray');
		},
		buildMenu() {
			const menu = new Menu();

			menu.append(new MenuItem(
				{
					label: 'Open LISTEN.moe',
					click: () => ipcRenderer.send('show-tray')
				}
			));
			menu.append(new MenuItem(
				{
					type: 'separator'
				}
			));
			menu.append(new MenuItem(
				{
					label: this.playing ? 'Pause' : 'Play',
					click: () => this.togglePlaying()
				}
			));
			menu.append(new MenuItem(
				{
					label: this.radioType === 'jpop' ? 'Switch to kpop' : 'Switch to jpop',
					click: () => this.$store.dispatch('setRadioType', this.radioType === 'jpop' ? 'kpop' : 'jpop')
				}
			));
			menu.append(new MenuItem(
				{
					label: (this.websocket && this.websocket.song && this.websocket.song.favorite) ? 'Unfavorite song' : 'Favorite song',
					click: () => this.toggleFavorite(),
					enabled: this.loggedIn
				}
			));
			menu.append(new MenuItem(
				{
					type: 'separator'
				}
			));
			menu.append(new MenuItem(
				{
					label: 'Settings',
					click: () => ipcRenderer.send('settingsModal')
				}
			));
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
					}
				}
			));
			menu.append(new MenuItem({ type: 'separator' }));
			menu.append(new MenuItem(
				{
					label: 'Quit',
					click: () => ipcRenderer.send('exit-tray')
				}
			));

			return menu;
		},
		setVolume(val) {
			const player = this.audio.audio;
			player.volume = val / 100;
			window.localStorage ? localStorage.setItem('volume', val / 100) : null; // eslint-disable-line
		},
		scrollVolume({ deltaY }) {
			const player = this.audio.audio;
			if (deltaY > 0) {
				if (player.volume === 0.0) return;
				player.volume = (player.volume - 0.1).toFixed(1);
			} else if (deltaY < 0) {
				if (player.volume === 1.0) return;
				player.volume = (player.volume + 0.1).toFixed(1);
			}
			window.localStorage ? localStorage.setItem('volume', player.volume) : null;
			this.volume = player.volume * 100;
		},
		updateDiscordActivity() {
			const artists = this.currentArtists.reduce((out, artist) => {
				out += `${this.preferRomaji ? artist.nameRomaji ? artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji : artist.name ? artist.name : artist.nameRomaji}${this.currentArtists.length > 1 ? ', ' : ''}`;
				return out;
			}, '');
			ipcRenderer.send('updateDiscordActivity', {
				details: this.currentSong.name.length >= 50 ? this.currentSong.name.substring(0, 50) : this.currentSong.name,
				state: artists.length >= 50 ? artists.substring(0, 50) : artists,
				startTimestamp: new Date(this.websocket.startTime).getTime(),
				endTimestamp: new Date(this.websocket.startTime).getTime() + new Date(this.websocket.song.duration * 1000).getTime(),
				largeImageKey: 'jpop',
				largeImageText: 'LISTEN.moe',
				smallImageKey: 'play',
				smallImageText: 'Playing',
				instance: false
			});
		},
		async checkFavorite() {
			if (!this.websocket) return;
			if (this.websocket.song.favorite) return;

			try {
				const { data } = await this.$apollo.query({
					query: checkFavorite,
					variables: { songs: [parseInt(this.websocket.song.id, 10)] }
				});

				if (!data.checkFavorite.length) {
					return;
				}
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}

			this.websocket.song.favorite = true;
			this.$forceUpdate();
		},
		togglePlaying() {
			if (!AUDIO_CONTEXT) {
				AUDIO_CONTEXT = new AudioContext();
				if (MEDIA_ELEMENT_NODES.has(this.audio.audio)) {
					SOURCE = MEDIA_ELEMENT_NODES.get(this.audio.audio);
				} else {
					SOURCE = AUDIO_CONTEXT.createMediaElementSource(this.audio.audio);
					MEDIA_ELEMENT_NODES.set(this.audio.audio, SOURCE);

					ANALYSER = AUDIO_CONTEXT.createAnalyser();

					SOURCE.connect(ANALYSER);
					ANALYSER.connect(AUDIO_CONTEXT.destination);
					ANALYSER.fftSize = FFT_SIZE;
					ANALYSER.minDecibels = MIN_DECIBELS;
					ANALYSER.maxDecibels = MAX_DECIBELS;
				}

				BUFFER_LENGTH = ANALYSER.frequencyBinCount;
				FREQUENCY_DATA = new Uint8Array(BUFFER_LENGTH);
			}

			const player = this.audio.audio;
			if (player.paused) {
				player.src = this.getSource();
				player.volume = window.localStorage ? localStorage.getItem('volume') ? localStorage.getItem('volume') : 0.5 : 0.5;
				player.play();
				MUSIC_VISUALS.start();
				this.$store.commit('playing', true);
				this.updateDiscordActivity();
				return;
			}
			player.pause();
			this.$store.commit('playing', false);
			MUSIC_VISUALS.stop();
			player.currentTime = 0;
			player.src = '';
			ipcRenderer.send('clearDiscordActivity');
		},
		getSource() {
			const isKpop = this.radioType === 'kpop' ? 'kpop/' : '';
			return `https://listen.moe/${isKpop}stream`;
		},
		async toggleFavorite() {
			if (!this.loggedIn) return ipcRenderer.send('loginModal');
			try {
				await this.$apollo.mutate({
					mutation: favoriteSong,
					variables: {
						id: this.websocket.song.id
					}
				});
				this.websocket.song.favorite = !Boolean(this.websocket.song.favorite);
				this.buildTray();
				this.$forceUpdate();
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}
		},
		buildTray() {
			if (!this.tray) this.tray = new Tray(join(__static, 'logo-trans.png'));
			this.tray.setToolTip('LISTEN.moe');
			this.tray.setContextMenu(this.buildMenu());
		}
	}
};
</script>
