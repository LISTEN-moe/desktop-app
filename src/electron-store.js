import electron from 'electron';
import path from 'path';
import fs from 'fs';

class Store {
	constructor(opts) {
		const userDataPath = (electron.app || electron.remote.app).getPath('userData');
		this.path = path.join(userDataPath, 'settings.json');
		this.data = this.parseDataFile(this.path, opts.defaults);
	}

	get(key) {
		return this.data[key];
	}

	set(key, val) {
		this.data[key] = val;
		fs.writeFile(this.path, JSON.stringify(this.data), err => {
			if (err) throw err;
		});
	}

	parseDataFile(filePath, defaults) {
		try {
			return JSON.parse(fs.readFileSync(filePath));
		} catch (error) {
			return defaults;
		}
	}
}

export default Store;
