module.exports = {
	productionSourceMap: false,
	pluginOptions: {
		electronBuilder: {
			builderOptions: {
				appId: 'moe.listen.desktop',
				productName: 'LISTEN.moe - Desktop App',
				copyright: 'Copyright © 2018-2019 iCrawl',
				appImage: {
					systemIntegration: 'doNotAsk'
			  },
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
					icon: 'build/256x256.png'
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
				}
			}
		}
	}
};
