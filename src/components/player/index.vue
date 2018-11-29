<style lang="scss" scoped>
	@import "@/assets/styles/_colors.scss";

	div.playerContainer {
		display: grid;
		grid-template-columns: 64px 1fr 64px 64px auto;
		grid-template-rows: auto 64px auto;
		grid-template-areas:
			". . . . coverArt"
			"playButton player favoriteButton volumeButton coverArt"
			". . . . coverArt";
		width: 100vw;

		&.gaps { grid-gap: 5px; }

		.playButton { grid-area: playButton; }
		.player {
			grid-area: player;
			-webkit-app-region: drag;
			position: relative;
			display: grid;
			grid-template-columns: 100%;
			grid-template-rows: auto auto auto;

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
				padding-top: 5px;
				text-align: left;
			}

			.titleContainer {
				text-align: left;
			}

			.info {
				max-height: 22px;
				background: linear-gradient(to right, rgba(0,0,0,0) 0%,rgba(0,0,0,0.3) 22%,rgba(0,0,0,0.3) 50%,rgba(0, 0, 0, 0.3) 80%,rgba(0,0,0,0) 100%);
				.requestedBy, .eventTime {
					font-size: .8rem;
					& span, a {
						font-size: .8rem;
					}
					& a { font-weight: 700; }
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
		}
		.favoriteButton { grid-area: favoriteButton; }
		.volumeButton { grid-area: volumeButton; }
		.albumContainer {
			grid-area: coverArt;
			max-width: 64px;
			background: none;
			a {
				display: block;
				max-height: 64px;

				img { max-width: 64px; }
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

</style>

<template>
	<div class="playerContainer"
		:class="{ gaps: enableGaps }">
		<div class="playButton shadow">
			<Budicon :icon="playing ? 'pause' : 'play'"
				y="-2"
				@click.native.stop.prevent="togglePlaying" />
		</div>
		<div class="player shadow">
			<canvas ref="canvas" />
			<Marquee v-if="currentArtists"
				ref="marquee"
				class="artistContainer">
				<span class="ja player-song-artist-container">
					<span v-for="(artist, index) in currentArtists"
						:key="artist.id"
						class="player-song-artist">
						<a :href="`https://beta.listen.moe/artists/${artist.id}`">
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
			<div class="info">
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
		<div class="favoriteButton shadow">
			<Budicon v-if="websocket"
				:icon="websocket.song.favorite ? 'filledStar' : 'star'"
				@click.native="toggleFavorite" />
		</div>
		<div class="volumeButton shadow">
			<Budicon icon="volume"
				@click.native="setVolume(100)" />
		</div>

		<div class="albumContainer"
			:class="{ big: !smallAlbumArt }">
			<div v-if="currentAlbum && albumCover"
				class="pictureContainer shadow">
				<a :href="currentAlbum.link">
					<img class="shadow"
						:src="`https://cdn.listen.moe/covers/${albumCover}`">
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
import { ipcRenderer } from 'electron';

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
		Marquee
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
			isVolumeSliderOpen: false
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
		currentRequester() {
			if (this.websocket && this.websocket.requester) {
				return {
					name: this.websocket.requester.displayName,
					link: `https://beta.listen.moe/u/${this.websocket.requester.username}`
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

				source.link = `https://beta.listen.moe/sources/${this.websocket.song.sources[0].id}`;
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

				album.link = `https://beta.listen.moe/albums/${this.websocket.song.albums[0].id}`;
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
		}
	},
	watch: {
		async websocket() {
			if (this.loggedIn) {
				await this.checkFavorite();
			}
			if (this.$refs && this.$refs.slider) this.$nextTick(() => this.$refs.slider.refresh());
		}
	},
	mounted() {
		this.volume = process.browser ? window.localStorage ? localStorage.getItem('volume') ? localStorage.getItem('volume') * 100 : 50 : 50 : 50;

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
	methods: {
		setVolume(val) {
			ipcRenderer.send('settingsModal');
			const player = this.audio.audio;
			player.volume = val / 100;
			process.browser ? window.localStorage ? localStorage.setItem('volume', val / 100) : null : null; // eslint-disable-line
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
			if (!this.loggedIn) ipcRenderer.send('loginModal');
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
				player.volume = process.browser ? window.localStorage ? localStorage.getItem('volume') ? localStorage.getItem('volume') : 0.5 : 0.5 : 0.5;
				player.play();
				MUSIC_VISUALS.start();
				this.$store.commit('playing', true);
				return;
			}
			player.pause();
			this.$store.commit('playing', false);
			MUSIC_VISUALS.stop();
			player.currentTime = 0;
			player.src = '';
		},
		getSource() {
			const isKpop = this.radioType === 'kpop' ? 'kpop/' : '';
			return `https://listen.moe/${isKpop}stream`;
		},
		async toggleFavorite() {
			try {
				await this.$apollo.mutate({
					mutation: favoriteSong,
					variables: {
						id: this.websocket.song.id
					}
				});
				this.websocket.song.favorite = !Boolean(this.websocket.song.favorite);
				this.$forceUpdate();
			} catch (error) {
				// TODO: Proper feedback
				/* this.$store.dispatch('alert', { message: error.message, error: true, duration: 5000 }); */
			}
		}
	}
};
</script>
