module.exports = {
	productionSourceMap: false,
	pluginOptions: {
		electronBuilder: {
			builderOptions: {
				appId: 'moe.listen.desktop',
				productName: 'LISTEN.moe',
				copyright: 'Copyright © 2018-2019 iCrawl',
				artifactName: '${productName}${arch}${version}.${ext}',
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
				},
				appImage: {
					systemIntegration: 'ask',
					category: 'Audio'
				}
			}
		}
	}
};
