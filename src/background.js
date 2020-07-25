import { app, protocol, BrowserWindow, shell, ipcMain } from 'electron';
import { createProtocol } from 'vue-cli-plugin-electron-builder/lib';
const isDevelopment = process.env.NODE_ENV !== 'production';
import Store from './electron-store';
import { join } from 'path';
import os from 'os';

const { Client } = require('discord-rpc');
const rpc = new Client({ transport: 'ipc' });

app.commandLine.appendSwitch('disable-features', 'OutOfBlinkCors');

// Global reference because javascript GC
let win;
let loginModal;
let settingsModal;
let minimizeToTray;

protocol.registerSchemesAsPrivileged([{ scheme: 'app', privileges: { secure: true, standard: true } }]);
async function createWindow() {
	const store = new Store({
		defaults: {
			windowSize: [800, 64],
			windowPosition: [null, null]
		}
	});

	const size = store.get('windowSize');
	const pos = store.get('windowPosition');

	win = new BrowserWindow({
		title: 'LISTEN.moe',
		// eslint-disable-next-line no-undef
		icon: join(__static, 'logo.png'),
		width: size[0],
		minWidth: 400,
		height: 64,
		minHeight: 64,
		x: pos[0],
		y: pos[1],
		frame: false,
		transparent: true,
		webPreferences: {
			webSecurity: false,
			nodeIntegration: true
		}
	});

	if (isDevelopment || process.env.IS_TEST) {
		win.loadURL(process.env.WEBPACK_DEV_SERVER_URL);
	} else {
		createProtocol('app');
		win.setMenuBarVisibility(false);
		win.loadURL('app://./index.html');
	}

	win.once('ready-to-show', () => {
		win.show();
		win.focus();
	});

	win.on('minimize', event => {
		if (!minimizeToTray) return;
		event.preventDefault();
		win.hide();
	});

	ipcMain.on('show-tray', () => {
		win.show();
		win.focus();
	});

	ipcMain.on('hide-tray', () => win.hide());
	ipcMain.on('exit-tray', () => app.quit());

	win.on('closed', () => {
		win = null;
	});

	win.on('resize', async () => {
		const values = win.getSize();
		await store.set('windowSize', values);
	});

	win.on('move', async () => {
		const values = win.getPosition();
		if (values[1] < 0) values[1] = 0;
		// Negative Y-Coordinate rebound happens automatically already, but seems to cause a syntax error in the settings json.
		// Therefore it is manually set to 0 before being saved to the settings.
		await store.set('windowPosition', values);
	});

	win.webContents.on('will-navigate', (event, url) => {
		event.preventDefault();
		shell.openExternal(url);
	});

	ipcMain.on('updateDiscordActivity', (_, arg) => rpc.setActivity(arg));
	ipcMain.on('clearDiscordActivity', () => rpc.clearActivity());

	ipcMain.on('loginModal', () => {
		if (loginModal) return loginModal.show();

		loginModal = new BrowserWindow({
			width: 350,
			height: 400,
			// eslint-disable-next-line no-undef
			icon: join(__static, 'logo.png'),
			frame: false,
			transparent: true,
			parent: win,
			webPreferences: {
				webSecurity: false,
				nodeIntegration: true
			}
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
			height: 760,
			// eslint-disable-next-line no-undef
			icon: join(__static, 'logo.png'),
			frame: false,
			transparent: true,
			parent: win,
			webPreferences: {
				webSecurity: false,
				nodeIntegration: true
			}
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

	ipcMain.on('login', (_, arg) => {
		win.webContents.send('login', arg);
		if (settingsModal) settingsModal.webContents.send('login', arg);
	});
	ipcMain.on('settingsChange', (_, arg) => {
		if (arg[0] === 'minimizeToTray') minimizeToTray = arg[1];
		win.webContents.send('playerOptionsChange', arg);
	});

	if (process.platform === 'win32' && os.release().startsWith('10')) {
		try {
			const { win10Controls } = require('./win10');
			win10Controls(ipcMain, win.webContents);
		} catch {}
	}

	await rpc.login({ clientId: '383375119827075072' });
}

// Disable hardware acceleration on Linux for transparent background
if (process.platform === 'linux') app.disableHardwareAcceleration();

app.on('window-all-closed', () => {
	if (process.platform !== 'darwin') app.quit();
});

app.on('activate', async () => {
	if (win === null) await createWindow();
});

app.on('ready', async () => {
	/* if (isDevelopment && !process.env.IS_TEST) await installVueDevtools(); */

	// Short timeout for Linux to make transparent background work.
	if (process.platform === 'linux') setTimeout(() => createWindow(), 300);
	else await createWindow();
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
