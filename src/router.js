import Vue from 'vue';
import Router from 'vue-router';
import Home from './views/index.vue';
import Login from './views/Login.vue';

Vue.use(Router);

export default new Router({
	mode: 'hash',
	routes: [
		{
			path: '/',
			name: 'home',
			component: Home
		},
		{
			path: '/login',
			name: 'login',
			component: Login
		}
	]
});
