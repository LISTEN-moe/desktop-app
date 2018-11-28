let ws = new WebSocket('wss://listen.moe/gateway');
let heartbeatInterval;

function heartbeat(interval) {
	heartbeatInterval = setInterval(() => {
		ws.send(JSON.stringify({ op: 9 }));
	}, interval);
}

ws.onopen = () => {
	console.log('%c> Websocket connection established.', 'color: #008000;');
	clearInterval(heartbeatInterval);
	ws.send(JSON.stringify({ op: 0, d: { auth: '' } }));
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
	/* if (!error.wasClean) */ setTimeout(() => ws = new WebSocket('wss://listen.moe/gateway'), 5000);
};
