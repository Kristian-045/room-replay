<script lang="ts">
  import { onMount } from 'svelte';
  import { Player, type PlaybackState } from './controller';
  import { save, savedPosition } from './progress';
  import { isActive, type Recording } from '../api';

  let { recording }: { recording: Recording } = $props();
  let video: HTMLVideoElement;
  let frame: HTMLDivElement;
  let controls: HTMLDivElement;
  let controller: Player | undefined;
  let playback = $state<PlaybackState>({ speed: 1, message: '', error: '' });
  let paused = $state(true);
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);
  let muted = $state(false);
  let fullscreen = $state(false);
  let visible = $state(true);
  let controlError = $state('');
  let hideTimer: ReturnType<typeof setTimeout>;
  const length = $derived(Number.isFinite(duration) ? Math.max(0, duration) : recording.duration);
  const showControls = $derived(visible || paused || !!playback.error || !!controlError);
  const controlButton = 'grid size-9 shrink-0 place-items-center rounded border-0 bg-transparent p-1.5 text-white hover:bg-white/15 focus-visible:outline-white';
  const textButton = 'shrink-0 rounded border-0 bg-transparent px-2 py-2 text-xs text-white hover:bg-white/15 focus-visible:outline-white';

  function time(value: number) {
    if (!Number.isFinite(value)) return '0:00';
    const seconds = Math.max(0, Math.floor(value));
    const hours = Math.floor(seconds / 3600);
    return hours ? `${hours}:${String(Math.floor(seconds / 60) % 60).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}` : `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
  }
  function reveal() {
    visible = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      if (controls?.matches(':hover') || controls?.querySelector(':focus-visible, select:focus')) reveal();
      else visible = false;
    }, 2500);
  }
  async function togglePlay() {
    controlError = '';
    try { if (video.paused) await video.play(); else video.pause(); }
    catch { controlError = 'Playback could not start. Try Play again.'; }
    reveal();
  }
  function seek(position: number) {
    if (length > 0) video.currentTime = Math.max(0, Math.min(position, length));
    reveal();
  }
  async function toggleFullscreen() {
    controlError = '';
    try {
      if (document.fullscreenElement === frame) await document.exitFullscreen();
      else await frame.requestFullscreen();
    } catch { controlError = 'Fullscreen is unavailable in this browser.'; }
    reveal();
  }
  const speeds = [0.5, 1, 1.25, 1.5, 2];
  function changeSpeed(direction: number) {
    const next = direction > 0
      ? speeds.find(speed => speed > video.playbackRate)
      : [...speeds].reverse().find(speed => speed < video.playbackRate);
    if (next !== undefined) controller?.speed(next);
    reveal();
  }
  function keyboard(event: KeyboardEvent) {
    if (!frame?.contains(document.activeElement) || event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
    const key = event.key.toLowerCase();
    if (key === 'k') { event.preventDefault(); void togglePlay(); }
    else if (key === 'arrowleft' || key === 'arrowright') { event.preventDefault(); seek(currentTime + (key === 'arrowleft' ? -5 : 5)); }
    else if (key === 'arrowup' || key === 'arrowdown') { event.preventDefault(); changeSpeed(key === 'arrowup' ? 1 : -1); }
    else if (key === 'f') { event.preventDefault(); void toggleFullscreen(); }
    else if (key === 'm') { event.preventDefault(); video.muted = !video.muted; reveal(); }
  }
  onMount(() => {
    const position = savedPosition(recording.id);
    let restored = false;
    const restore = () => {
      if (restored || !Number.isFinite(video.duration) || video.duration <= 0) return;
      video.currentTime = Math.min(position, Math.max(0, video.duration - 0.1));
      restored = true;
    };
    const persist = () => {
      if (restored && Number.isFinite(video.currentTime)) save('position:' + recording.id, String(video.currentTime));
    };
    video.addEventListener('loadedmetadata', restore);
    video.addEventListener('durationchange', restore);
    video.addEventListener('timeupdate', persist);
    video.addEventListener('seeked', persist);
    window.addEventListener('pagehide', persist);
    controller = new Player(video, `/media/${recording.id}/index.m3u8`, state => playback = state, position);
    const fullscreenChanged = () => { fullscreen = document.fullscreenElement === frame; reveal(); };
    document.addEventListener('fullscreenchange', fullscreenChanged);
    return () => {
      persist();
      video.removeEventListener('loadedmetadata', restore);
      video.removeEventListener('durationchange', restore);
      video.removeEventListener('timeupdate', persist);
      video.removeEventListener('seeked', persist);
      window.removeEventListener('pagehide', persist);
      clearTimeout(hideTimer);
      document.removeEventListener('fullscreenchange', fullscreenChanged);
      controller?.destroy();
    };
  });
</script>

<svelte:window onkeydown={keyboard} />
<div class="px-3 pb-3 sm:px-5 sm:pb-5">
  <div bind:this={frame} role="group" aria-label="Video player" data-player-frame class="relative aspect-video w-full overflow-hidden rounded-lg bg-black text-white [&:fullscreen]:h-screen [&:fullscreen]:w-screen [&:fullscreen]:rounded-none" onpointermove={reveal} onpointerdown={reveal} onfocusin={reveal}>
    <!-- The source does not provide captions. -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <video class="h-full w-full object-contain" bind:this={video} bind:paused bind:currentTime bind:duration bind:volume bind:muted playsinline preload="auto" onplay={reveal}></video>
    <button class="absolute inset-0 grid h-full w-full place-items-center rounded-none border-0 bg-transparent p-0 text-white hover:bg-transparent focus-visible:outline-none" aria-label={paused ? 'Play video' : 'Pause video'} onclick={togglePlay}>
      {#if paused}<span class="grid size-14 place-items-center rounded-full bg-black/60"><svg aria-hidden="true" viewBox="0 0 24 24" class="ml-1 size-7 fill-current"><path d="M7 4v16l14-8z" /></svg></span>{/if}
    </button>
    {#if playback.error || controlError}<p role="alert" class="absolute inset-x-3 top-3 rounded bg-red-950/90 px-3 py-2 text-xs text-white">{playback.error || controlError}</p>{/if}
    <div bind:this={controls} data-player-controls class={["absolute inset-x-0 bottom-0 bg-linear-to-t from-black/95 via-black/75 to-transparent px-2 pb-2 pt-8 transition-opacity duration-200 sm:px-3", showControls ? 'opacity-100' : 'pointer-events-none opacity-0']}>
      {#if playback.message}<p role="status" class="mb-1 text-xs text-white/90">{playback.message}</p>{/if}
      <input aria-label="Seek" aria-valuetext={`${time(currentTime)} of ${time(length)}`} type="range" min="0" max={Math.max(length, 0.01)} step="0.1" value={currentTime} disabled={length <= 0} oninput={event => seek(Number(event.currentTarget.value))} class="block h-5 w-full cursor-pointer rounded-none border-0 bg-transparent p-0 accent-violet-400 focus-visible:outline-white" />
      <div class="flex flex-wrap items-center gap-x-1 gap-y-0.5 sm:gap-x-2">
        <button class={controlButton} aria-label={paused ? 'Play' : 'Pause'} title={paused ? 'Play (K)' : 'Pause (K)'} onclick={togglePlay}>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-current">{#if paused}<path d="M7 4v16l14-8z" />{:else}<path d="M5 4h5v16H5zm9 0h5v16h-5z" />{/if}</svg>
        </button>
        <button class={controlButton} aria-label={muted || volume === 0 ? 'Unmute' : 'Mute'} title="Mute (M)" onclick={() => { video.muted = !video.muted; if (video.volume === 0) video.volume = 0.5; reveal(); }}>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-none stroke-current" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4 6 8H3v8h3l5 4z" />{#if muted || volume === 0}<path d="m16 9 6 6m0-6-6 6" />{:else}<path d="M15 8a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14" />{/if}</svg>
        </button>
        <input aria-label="Volume" type="range" min="0" max="1" step="0.05" value={muted ? 0 : volume} oninput={event => { video.volume = Number(event.currentTarget.value); video.muted = false; reveal(); }} class="hidden h-5 w-16 cursor-pointer rounded-none border-0 bg-transparent p-0 accent-white sm:block" />
        <span class="mr-auto whitespace-nowrap text-[10px] tabular-nums text-white/90 sm:text-xs">{time(currentTime)} / {time(length)}</span>
        {#if isActive(recording)}<button class={textButton} onclick={() => { controller?.goLive(); reveal(); }}>Go live</button>{/if}
        <div role="group" aria-label="Playback speed" class="flex shrink-0 items-center rounded bg-black/50">
          <button class={controlButton} aria-label="Decrease speed" title="Slower (↓)" disabled={playback.speed <= speeds[0]} onclick={() => changeSpeed(-1)}>−</button>
          <span class="min-w-9 text-center text-xs tabular-nums" aria-live="polite">{playback.speed}×</span>
          <button class={controlButton} aria-label="Increase speed" title="Faster (↑)" disabled={playback.speed >= speeds[speeds.length - 1]} onclick={() => changeSpeed(1)}>+</button>
        </div>
        <button class={controlButton} aria-label={fullscreen ? 'Exit fullscreen' : 'Fullscreen'} title="Fullscreen (F)" onclick={toggleFullscreen}>
          <svg aria-hidden="true" viewBox="0 0 24 24" class="size-5 fill-none stroke-current" stroke-width="2">{#if fullscreen}<path d="M9 3v6H3m18 0h-6V3M3 15h6v6m6 0v-6h6" />{:else}<path d="M9 3H3v6m12-6h6v6M3 15v6h6m6 0h6v-6" />{/if}</svg>
        </button>
      </div>
    </div>
  </div>
</div>
