import electron from 'electron';
import path from 'path';
import fs from 'fs';
import { promisify } from 'util';

const writeFile = promisify(fs.writeFile);

export default class Store {
	constructor(opts) {
		const userDataPath = (electron.app || electron.remote.app).getPath('userData');
		this.path = path.join(userDataPath, 'settings.json');
		this.data = this.parseDataFile(this.path, opts.defaults);
	}

	get(key) {
		return this.data[key];
	}

	async set(key, val) {
		this.data[key] = val;
		try {
			await writeFile(this.path, JSON.stringify(this.data));
		} catch (error) {
			console.error(error);
		}
	}

	parseDataFile(filePath, defaults) {
		try {
			return JSON.parse(fs.readFileSync(filePath));
		} catch (error) {
			return defaults;
		}
	}
}
