import Vue from 'vue';
import VueApollo from 'vue-apollo';
import App from './App.vue';

import router from './router';
import store from './store';
import { createProvider } from './vue-apollo';

Vue.config.productionTip = false;

Vue.use(VueApollo);

(async () => {
	new Vue({
		router,
		store,
		apolloProvider: await createProvider(),
		render: h => h(App)
	}).$mount('#app');
})();

