import Vue from 'vue';
import Vuex, { Store } from 'vuex';

Vue.use(Vuex);

export default new Store({
	state: {
		websocket: null,
		playing: false
	},
	mutations: {
		websocket(state, payload) {
			state.websocket = payload;
		},
		playing(state, payload) {
			state.playing = payload;
		}
	},
	actions: {

	}
});
