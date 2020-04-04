let heartbeatInterval;
let gateway = 'gateway_v2';
let forced = false;
let ws;

function heartbeat(interval) {
	heartbeatInterval = setInterval(() => {
		ws.send(JSON.stringify({ op: 9 }));
	}, interval);
}

function connect() {
	ws = new WebSocket(`wss://listen.moe/${gateway}`);

	ws.onopen = () => {
		console.log('%c> Websocket connection established.', 'color: #008000;');
		clearInterval(heartbeatInterval);
		heartbeatInterval = null;
	};

	ws.onmessage = message => {
		if (!message.data.length) return;
		let response;
		try {
			response = JSON.parse(message.data);
		} catch (error) {
			return;
		}
		switch (response.op) {
			case 0:
				ws.send(JSON.stringify({ op: 9 }));
				heartbeat(response.d.heartbeat);
				break;
			case 1:
				if (response.t !== 'TRACK_UPDATE' && response.t !== 'TRACK_UPDATE_REQUEST' && response.t !== 'QUEUE_UPDATE' && response.t !== 'NOTIFICATION') break;
				postMessage(response.d);
				break;
			default:
				break;
		}
	};

	ws.onclose = error => {
		console.log('%c> Websocket connection closed. Reconnecting...', 'color: #ff015b;', error);
		clearInterval(heartbeatInterval);
		heartbeatInterval = null;
		if (ws) {
			ws.close();
			ws = null;
		}
		setTimeout(() => connect(), forced ? 1000 : 5000);
		forced = false;
	};
}

onmessage = function(message) {
	gateway = message.data === 'jpop' ? 'gateway_v2' : 'kpop/gateway_v2';
	forced = true;
	if (ws) ws.close();
};

connect();
