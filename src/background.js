import { app, protocol, BrowserWindow, shell, ipcMain } from 'electron';
import { createProtocol, installVueDevtools } from 'vue-cli-plugin-electron-builder/lib';
const isDevelopment = process.env.NODE_ENV !== 'production';

// Global reference because javascript GC
let win;
let loginModal;
let settingsModal;

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

	win.once('ready-to-show', () => {
		win.show();
		win.focus();
	});

	win.on('closed', () => {
		win = null;
	});

	win.webContents.on('will-navigate', (event, url) => {
		event.preventDefault();
		shell.openExternal(url);
	});

	ipcMain.on('loginModal', () => {
		if (loginModal) {
			return loginModal.show();
		}

		loginModal = new BrowserWindow({
			width: 350,
			height: 500,
			frame: false,
			transparent: true,
			parent: win,
			show: false
		});

		if (isDevelopment || process.env.IS_TEST) {
			loginModal.loadURL(`${process.env.WEBPACK_DEV_SERVER_URL}#/login`);
		} else {
			loginModal.loadURL('app://./index.html#login');
		}

		loginModal.once('ready-to-show', () => {
			loginModal.show();
		});

		loginModal.once('close', () => {
			loginModal = null;
		});
	});

	ipcMain.on('settingsModal', () => {
		if (settingsModal) {
			return settingsModal.show();
		}

		settingsModal = new BrowserWindow({
			width: 500,
			height: 500,
			frame: false,
			transparent: true,
			parent: win,
			show: false
		});

		if (isDevelopment || process.env.IS_TEST) {
			settingsModal.loadURL(`${process.env.WEBPACK_DEV_SERVER_URL}#/settings`);
		} else {
			settingsModal.loadURL('app://./index.html#settings');
		}

		settingsModal.once('ready-to-show', () => {
			settingsModal.show();
		});

		settingsModal.once('close', () => {
			settingsModal = null;
		});
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
