module.exports = {
	productionSourceMap: false,
	pluginOptions: {
		electronBuilder: {
			builderOptions: {
				appId: 'moe.listen.desktop',
				productName: 'LISTEN.moe - Desktop App',
				copyright: 'Copyright © 2018-2019 iCrawl',
				icon: 'build/256x256.png',
				win: {
					target: [
						{
							target: 'portable',
							arch: [
								'x64'
							]
						}
					]
				},
				mac: {
					target: [
						{
							target: 'dmg',
							arch: [
								'x64'
							]
						}
					]
				},
				linux: {
					target: [
						{
							target: 'AppImage',
							arch: [
								'x64'
							]
						}
					]
				}
			}
		}
	}
};
