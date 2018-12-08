import { app, protocol, BrowserWindow, shell, ipcMain } from 'electron';
import { createProtocol, installVueDevtools } from 'vue-cli-plugin-electron-builder/lib';
const isDevelopment = process.env.NODE_ENV !== 'production';

const { Client } = require('discord-rpc');
const rpc = new Client({ transport: 'ipc' });
import Store from './electron-store';

// Global reference because javascript GC
let win;
let loginModal;
let settingsModal;

protocol.registerStandardSchemes(['app'], { secure: true });
async function createWindow() {
	const store = new Store({
		defaults: {
			windowSize: [800, 80],
			windowPosition: []
		}
	});

	const size = store.get('windowSize');
	const pos = store.get('windowPosition');

	win = new BrowserWindow({
		title: 'LISTEN.MOE - Desktop App',
		width: size[0],
		minWidth: 400,
		height: 80,
		minHeight: 80,
		x: pos ? pos[0] ? pos[0] : null : null,
		y: pos ? pos[1] ? pos[1] : null : null,
		frame: false,
		transparent: true
	});

	if (isDevelopment || process.env.IS_TEST) {
		win.loadURL(process.env.WEBPACK_DEV_SERVER_URL);
	} else {
		createProtocol('app');
		win.loadURL('app://./index.html');
	}

	// win.webContents.openDevTools();

	win.once('ready-to-show', () => {
		win.show();
		win.focus();
	});

	win.on('closed', () => {
		win = null;
	});

	win.on('resize', () => {
		const values = win.getSize();
		store.set('windowSize', values);
	});

	win.on('move', () => {
		const values = win.getPosition();
		store.set('windowPosition', values);
	});

	win.webContents.on('will-navigate', (event, url) => {
		event.preventDefault();
		shell.openExternal(url);
	});

	await rpc.login({ clientId: '383375119827075072' });

	ipcMain.on('updateDiscordActivity', (_, arg) => rpc.setActivity(arg));

	ipcMain.on('loginModal', () => {
		if (loginModal) return loginModal.show();

		loginModal = new BrowserWindow({
			width: 350,
			height: 500,
			frame: false,
			transparent: true,
			parent: win
		});

		if (isDevelopment || process.env.IS_TEST) {
			loginModal.loadURL(`${process.env.WEBPACK_DEV_SERVER_URL}#/login`);
		} else {
			loginModal.loadURL('app://./index.html#login');
		}

		loginModal.once('close', () => {
			loginModal = null;
		});
	});

	ipcMain.on('settingsModal', () => {
		if (settingsModal) return settingsModal.show();

		settingsModal = new BrowserWindow({
			width: 500,
			height: 500,
			frame: false,
			transparent: true,
			parent: win
		});

		if (isDevelopment || process.env.IS_TEST) {
			settingsModal.loadURL(`${process.env.WEBPACK_DEV_SERVER_URL}#/settings`);
		} else {
			settingsModal.loadURL('app://./index.html#settings');
		}

		settingsModal.once('close', () => {
			settingsModal = null;
		});
	});

	ipcMain.on('reload', () => win.reload());

	ipcMain.on('login', (_, arg) => win.webContents.send('login', arg));
	ipcMain.on('settingsChange', (_, arg) => win.webContents.send('playerOptionsChange', arg));
}

app.on('window-all-closed', () => {
	if (process.platform !== 'darwin') app.quit();
});

app.on('activate', async () => {
	if (win === null) await createWindow();
});

app.on('ready', async () => {
	if (isDevelopment && !process.env.IS_TEST) await installVueDevtools();
	await createWindow();
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
