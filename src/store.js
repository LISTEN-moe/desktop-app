import Vue from 'vue';
import Vuex, { Store } from 'vuex';

Vue.use(Vuex);

export default new Store({
	state: {
		loggedIn: false,
		user: null,
		token: null,
		websocket: null,
		playing: false,
		isJpop: true,
		// Settings
		preferRomaji: false,
		eventStarts: true,
		enableGaps: true,
		smallAlbumArt: false
	},
	mutations: {
		loggedIn(state, payload) {
			state.loggedIn = payload;
		},
		user(state, payload) {
			if (!payload) {
				state.user = null;
				return;
			}

			for (const role of payload.roles) {
				switch (role.name) {
					case 'admin':
						payload.isAdmin = true;
						break;
					case 'judge':
						payload.isJudge = true;
						break;
					case 'idol':
						payload.isSupporter = true;
						break;
					case 'uploader':
						payload.isUploader = true;
						break;
					case 'editor':
						payload.isEditor = true;
						break;
					case 'developer':
						payload.isDeveloper = true;
						break;
					default:
						break;
				}
			}
			state.user = payload;
		},
		token(state, payload) {
			state.token = payload;
		},
		websocket(state, payload) {
			state.websocket = payload;
		},
		playing(state, payload) {
			state.playing = payload;
		},
		isJpop(state, payload) {
			state.isJpop = payload;
		},

		// Settings
		preferRomaji(state, payload) {
			state.preferRomaji = payload;
		},
		eventStarts(state, payload) {
			state.eventStarts = payload;
		},
		enableGaps(state, payload) {
			state.enableGaps = payload;
		},
		smallAlbumArt(state, payload) {
			state.smallAlbumArt = payload;
		}
	},
	actions: {
		login({ commit }, { token, user }) {
			commit('token', token);
			commit('user', user);
			commit('loggedIn', true);
		},
		logout({ commit }) {
			commit('token', null);
			commit('user', null);
			commit('loggedIn', false);
			localStorage.removeItem('token');
		},
		setState({ commit }, { option, value }) {
			commit(option, value);
			localStorage.setItem(option, value);
		},
		setInitialState({ dispatch }) {
			const whatToPopulate = ['preferRomaji', 'eventStarts', 'enableGaps', 'smallAlbumArt'];

			for (const key of whatToPopulate) {
				const storageValue = localStorage.getItem(key);
				if (storageValue && storageValue === 'false') dispatch('setState', { option: key, value: false });
				else dispatch('setState', { option: key, value: true });
			}
		}
	}
});
