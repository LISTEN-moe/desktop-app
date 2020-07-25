module.exports = {
	productionSourceMap: false,
	pluginOptions: {
		electronBuilder: {
			nodeIntegration: true,
			externals: [
				'@nodert-win10-rs4/windows.foundation',
				'@nodert-win10-rs4/windows.media',
				'@nodert-win10-rs4/windows.media.playback',
				'@nodert-win10-rs4/windows.storage.streams'
			],
			builderOptions: {
				appId: 'moe.listen.desktop',
				productName: 'LISTEN.moe',
				copyright: 'Copyright © 2018-2020 iCrawl',
				icon: 'build/512x512.png',
				win: {
					target: [
						{
							target: 'portable',
							arch: [
								'x64'
							]
						}
					],
					icon: 'build/icon.ico'
				},
				mac: {
					target: [
						{
							target: 'dmg',
							arch: [
								'x64'
							]
						}
					],
					icon: 'build/512x512.png'
				},
				linux: {
					target: [
						{
							target: 'AppImage',
							arch: [
								'x64'
							]
						}
					],
					category: 'Audio',
					icon: 'build/256x256.png'
				},
				appImage: {
					category: 'Audio'
				}
			}
		}
	}
};
