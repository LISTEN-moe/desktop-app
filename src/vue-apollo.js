import Vue from 'vue';
import VueApollo from 'vue-apollo';
import { createApolloClient, restartWebsockets } from 'vue-cli-plugin-apollo/graphql-client';

Vue.use(VueApollo);

const AUTH_TOKEN = 'token';

const defaultOptions = {
	httpEndpoint: 'https://listen.moe/graphql',
	wsEndpoint: 'wss://listen.moe/subscriptions',
	tokenName: 'token',
	persisting: false,
	websocketsOnly: false,
	ssr: false
};

export function createProvider(options = {}) {
	const { apolloClient, wsClient } = createApolloClient({
		...defaultOptions,
		...options
	});
	apolloClient.wsClient = wsClient;

	const apolloProvider = new VueApollo({
		defaultClient: apolloClient,
		defaultOptions: {
			$query: {
				fetchPolicy: 'no-cache'
			}
		},
		errorHandler(error) {
			console.log('%cError', 'background: red; color: white; padding: 2px 4px; border-radius: 3px; font-weight: bold;', error.message);
		}
	});

	return apolloProvider;
}

export async function onLogin(apolloClient, token) {
	if (typeof localStorage !== 'undefined' && token) {
		localStorage.setItem(AUTH_TOKEN, token);
	}
	if (apolloClient.wsClient) restartWebsockets(apolloClient.wsClient);
	try {
		await apolloClient.resetStore();
	} catch (e) {
		console.log('%cError on cache reset (login)', 'color: orange;', e.message);
	}
}

export async function onLogout(apolloClient) {
	if (typeof localStorage !== 'undefined') {
		localStorage.removeItem(AUTH_TOKEN);
	}
	if (apolloClient.wsClient) restartWebsockets(apolloClient.wsClient);
	try {
		await apolloClient.resetStore();
	} catch (e) {
		console.log('%cError on cache reset (logout)', 'color: orange;', e.message);
	}
}
