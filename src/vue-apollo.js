import VueApollo from 'vue-apollo';
import { ApolloClient } from 'apollo-client';
import { InMemoryCache, defaultDataIdFromObject } from 'apollo-cache-inmemory';
import { createUploadLink } from 'apollo-upload-client';
import { setContext } from 'apollo-link-context';
import { SubscriptionClient } from 'subscriptions-transport-ws';
import MessageTypes from 'subscriptions-transport-ws/dist/message-types';
import { WebSocketLink } from 'apollo-link-ws';
import { split } from 'apollo-link';
import { getMainDefinition } from 'apollo-utilities';
import { remote } from 'electron';

export async function createProvider() {
	const cookies = await remote.session.defaultSession.cookies.get({ url: 'https://listen.moe' });
	const presentCookie = cookies.find(c => c.name === 'token');
	let presentLocalStorage;
	if (typeof localStorage !== 'undefined') {
		presentLocalStorage = localStorage.getItem('token');
	}
	const authorization = presentCookie ? { authorization: `Bearer ${presentCookie.value}` } : presentLocalStorage ? { authorization: `Bearer ${presentLocalStorage}` } : {};

	const cache = new InMemoryCache({
		dataIdFromObject: object => {
			switch (object.__typename) {
				case 'Artist': return `artist:${object.id}`;
				case 'Album': return `album:${object.id}`;
				case 'Source': return `source:${object.id}`;
				default: return defaultDataIdFromObject(object);
			}
		}
	});
	const httpLink = createUploadLink({
		uri: 'https://listen.moe/graphql',
		credentials: 'include'
	});
	const authLink = setContext((_, { headers }) => ({
		headers: {
			...headers,
			...authorization
		}
	}));
	const wsClient = new SubscriptionClient('wss://listen.moe/subscriptions', {
		reconnect: true,
		lazy: true,
		connectionParams: () => ({ ...authorization })
	}); // eslint-disable-line
	wsClient.maxConnectTimeGenerator.setMin(6000);
	const wsLink = new WebSocketLink(wsClient);
	const apolloClient = new ApolloClient({
		link: split(({ query }) => {
			const { kind, operation } = getMainDefinition(query);
			return kind === 'OperationDefinition' && operation === 'subscription';
		}, wsLink, authLink.concat(httpLink)),
		cache,
		ssrMode: true,
		ssrForceFetchDelay: 100,
		connectToDevTools: process.env.NODE_ENV !== 'production', // eslint-disable-line no-undef
		defaultOptions: {
			query: {
				fetchPolicy: 'no-cache'
			}
		}
	});
	apolloClient.wsClient = wsClient;

	const apolloProvider = new VueApollo({
		defaultClient: apolloClient
	});

	return apolloProvider;
}

function restartWebsockets(client) {
	const operations = { ...client.operations };

	client.close(true);
	client.connect();

	Object.keys(operations).forEach(id => client.sendMessage(id, MessageTypes.GQL_START, operations[id].options));
}

export async function onLogin(apolloClient, token) {
	if (token) {
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem('token', token);
		}
		await remote.session.defaultSession.cookies.set({
			url: 'https://listen.moe',
			name: 'token',
			value: token,
			domain: 'listen.moe',
			expirationDate: new Date(Date.now() + (86400 * 1000 * 30))
		});
	}
	if (apolloClient.wsClient) restartWebsockets(apolloClient.wsClient);
}

export async function onLogout(apolloClient) {
	if (typeof localStorage !== 'undefined') {
		localStorage.removeItem('token');
	}
	await remote.session.defaultSession.cookies.remove('https://listen.moe', 'token');
	if (apolloClient.wsClient) restartWebsockets(apolloClient.wsClient);
}
