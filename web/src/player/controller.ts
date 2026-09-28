import Hls from 'hls.js';
import { livePosition, shouldReturnToNormal } from './policy';

export type PlaybackState = { speed: number; message: string; error: string };

export class Player {
  private hls: Hls | undefined;
  private edge = 0;
  private live = true;
  private state: PlaybackState = { speed: 1, message: '', error: '' };
  private mediaRecoveries = 0;

  constructor(private video: HTMLVideoElement, url: string, private changed: (state: PlaybackState) => void) {
    video.preservesPitch = true;
    video.addEventListener('timeupdate', this.onTime);
    video.addEventListener('ratechange', this.onRate);
    if (Hls.isSupported()) {
      this.hls = new Hls({
        startPosition: 0,
        liveSyncDurationCount: 3,
        liveMaxLatencyDurationCount: Infinity,
        maxLiveSyncPlaybackRate: 1,
        liveDurationInfinity: false,
        backBufferLength: 30,
        maxBufferLength: 30,
      });
      this.hls.on(Hls.Events.LEVEL_UPDATED, (_, data) => {
        this.edge = data.details.edge;
        this.live = data.details.live;
      });
      this.hls.on(Hls.Events.ERROR, (_, data) => {
        if (!data.fatal) return;
        if (data.type === Hls.ErrorTypes.MEDIA_ERROR && this.mediaRecoveries++ < 2) {
          this.hls?.recoverMediaError();
        } else {
          this.state.error = 'Playback was interrupted. Close and reopen this recording to retry.';
          this.emit();
        }
      });
      this.hls.attachMedia(video);
      this.hls.loadSource(url);
    } else {
      this.state.error = 'This browser does not support the required video playback.';
      this.emit();
    }
  }

  private emit() { this.changed({ ...this.state }); }
  private onRate = () => { this.state.speed = this.video.playbackRate; this.emit(); };
  private onTime = () => {
    if (shouldReturnToNormal(this.video.playbackRate, this.video.currentTime, this.edge, this.live)) {
      this.video.playbackRate = 1;
      this.state.message = 'Caught up. Playing at 1×.';
      this.emit();
    }
  };

  speed(value: number) { this.state.message = ''; this.video.playbackRate = value; this.emit(); }
  startOver() { this.video.currentTime = 0; }
  goLive() { this.video.currentTime = livePosition(this.edge); this.video.playbackRate = 1; }
  destroy() {
    this.video.removeEventListener('timeupdate', this.onTime);
    this.video.removeEventListener('ratechange', this.onRate);
    this.hls?.destroy();
    this.video.removeAttribute('src');
    this.video.load();
  }
}
