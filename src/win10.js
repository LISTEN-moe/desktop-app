import { MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControlsButton } from '@nodert-win10-rs4/windows.media';
import { BackgroundMediaPlayer } from '@nodert-win10-rs4/windows.media.playback';
import { RandomAccessStreamReference } from '@nodert-win10-rs4/windows.storage.streams';
import { Uri } from '@nodert-win10-rs4/windows.foundation';

const Controls = BackgroundMediaPlayer.current.systemMediaTransportControls;

Controls.isChannelDownEnabled = false;
Controls.isChannelUpEnabled = false;
Controls.isFastForwardEnabled = false;
Controls.isNextEnabled = false;
Controls.isPauseEnabled = true;
Controls.isPlayEnabled = true;
Controls.isPreviousEnabled = false;
Controls.isRecordEnabled = false;
Controls.isRewindEnabled = false;
Controls.isStopEnabled = false;
Controls.isEnabled = true;
Controls.playbackStatus = MediaPlaybackStatus.closed;
Controls.displayUpdater.type = MediaPlaybackType.music;

Controls.displayUpdater.musicProperties.title = 'LISTEN.moe';
Controls.displayUpdater.musicProperties.artist = 'No track playing.';
Controls.displayUpdater.update();

export function win10Controls(ipc, webContents) {
	let playing = false;

	Controls.on('buttonpressed', (_, eventArgs) => {
		switch (eventArgs.button) {
			case SystemMediaTransportControlsButton.play:
				if (!playing) webContents.send('togglePlaying');
				break;
			case SystemMediaTransportControlsButton.pause:
				if (playing) webContents.send('togglePlaying');
				break;
			case SystemMediaTransportControlsButton.stop:
				if (playing) webContents.send('togglePlaying');
				break;
			default:
				break;
		}
	});

	ipc.on('changeTrack', (_, arg) => {
		Controls.displayUpdater.musicProperties.title = arg.title;
		Controls.displayUpdater.musicProperties.artist = arg.artist;
		Controls.displayUpdater.musicProperties.albumTitle = arg.album;
		Controls.displayUpdater.musicProperties.thumbnail = RandomAccessStreamReference.createFromUri(new Uri(arg.albumImage));
		Controls.displayUpdater.update();
	});

	ipc.on('changePlaying', (_, arg) => {
		playing = arg.playing;

		if (playing) {
			Controls.playbackStatus = MediaPlaybackStatus.playing;
		} else {
			Controls.playbackStatus = MediaPlaybackStatus.paused;
		}
		Controls.displayUpdater.update();
	});
}
