import { app, protocol, BrowserWindow, shell } from 'electron';
import { createProtocol, installVueDevtools } from 'vue-cli-plugin-electron-builder/lib';
const isDevelopment = process.env.NODE_ENV !== 'production';

// Global reference because javascript GC
let win;

protocol.registerStandardSchemes(['app'], { secure: true });
function createWindow() {
	win = new BrowserWindow({
		width: 800,
		height: 200,
		frame: false,
		transparent: true
	});

	if (isDevelopment || process.env.IS_TEST) {
		win.loadURL(process.env.WEBPACK_DEV_SERVER_URL);
	} else {
		createProtocol('app');
		win.loadURL('app://./index.html');
	}

	win.on('closed', () => {
		win = null;
	});

	win.webContents.on('will-navigate', (event, url) => {
		event.preventDefault();
		shell.openExternal(url);
	});
}

app.on('window-all-closed', () => {
	if (process.platform !== 'darwin') app.quit();
});

app.on('activate', () => {
	if (win === null) createWindow();
});

app.on('ready', async () => {
	if (isDevelopment && !process.env.IS_TEST) await installVueDevtools();
	createWindow();
});

if (isDevelopment) {
	if (process.platform === 'win32') {
		process.on('message', data => {
			if (data === 'graceful-exit') app.quit();
		});
	} else {
		process.on('SIGTERM', () => app.quit());
	}
}
