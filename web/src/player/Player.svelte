<script lang="ts">
  import { onMount } from 'svelte';
  import { Player, type PlaybackState } from './controller';
  import { isActive, type Recording } from '../api';

  let { recording }: { recording: Recording } = $props();
  let video: HTMLVideoElement;
  let controller: Player | undefined;
  let playback = $state<PlaybackState>({ speed: 1, message: '', error: '' });
  onMount(() => {
    controller = new Player(video, `/media/${recording.id}/index.m3u8`, state => playback = state);
    return () => controller?.destroy();
  });
</script>

<div class="px-3 pb-5 sm:px-5">
  <!-- Captions are not supplied by the source. -->
  <!-- svelte-ignore a11y_media_has_caption -->
  <video class="aspect-video w-full rounded-lg bg-stone-950" bind:this={video} controls playsinline preload="auto"></video>
  <div class="flex flex-wrap items-center justify-between gap-3 pt-4">
    <div class="flex gap-2">
      <button onclick={() => controller?.startOver()}>Start over</button>
      {#if isActive(recording)}<button onclick={() => controller?.goLive()}>Go live</button>{/if}
    </div>
    <label class="flex flex-row items-center gap-2 [&_select]:w-20">Speed
      <select aria-label="Playback speed" value={playback.speed} onchange={event => controller?.speed(Number(event.currentTarget.value))}>
        {#each [0.5, 1, 1.25, 1.5, 2] as speed}<option value={speed}>{speed}×</option>{/each}
      </select>
    </label>
  </div>
  {#if playback.message}<p role="status" class="mt-3 text-xs text-brand">{playback.message}</p>{/if}
  {#if playback.error}<p role="alert" class="mt-3 text-xs text-red-700">{playback.error}</p>{/if}
</div>
