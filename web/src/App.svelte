<script lang="ts">
  import { onMount } from 'svelte';
  import { request, isActive, type Snapshot } from './api';
  import Player from './player/Player.svelte';

  let snapshot = $state<Snapshot>({ rooms: [], recordings: [] });
  let error = $state('');
  let busy = $state(false);
  let connected = $state(false);
  let roomName = $state('');
  let roomUrl = $state('');
  let selectedRoom = $state('');
  let selectedRecording = $state('');
  let showRoomForm = $state(false);
  let endTime = $state(localInput(Date.now() + 60 * 60 * 1000));
  let extensionTime = $state('');
  const active = $derived(snapshot.recordings.find(isActive));
  const playable = $derived(snapshot.recordings.filter(recording => recording.playable || isActive(recording)));
  const history = $derived(snapshot.recordings.filter(recording => !recording.playable && !isActive(recording)));
  const selected = $derived(snapshot.recordings.find(recording => recording.id === selectedRecording));

  function localInput(time: number) {
    const date = new Date(time);
    return new Date(time - date.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
  }
  const clock = (time: number) => new Date(time * 1000).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  const minutes = (duration: number) => `${Math.floor(duration / 60)}m ${Math.floor(duration % 60)}s`;
  const label = (phase: string) => ({ no_stream: 'No stream', recording: 'Recording', waiting: 'Connecting', retrying: 'Retrying', finished: 'Finished', stopped: 'Stopped', failed: 'Failed' }[phase] ?? phase);

  async function refresh() {
    try { snapshot = await request(); connected = true; }
    catch { connected = false; }
  }
  async function mutate(path: string, body: unknown) {
    busy = true; error = '';
    try { snapshot = await request(path, body); connected = true; return true; }
    catch (failure) { error = failure instanceof Error ? failure.message : 'Request failed'; return false; }
    finally { busy = false; }
  }
  async function saveRoom(event: SubmitEvent) {
    event.preventDefault();
    if (await mutate('/api/rooms', { name: roomName, url: roomUrl })) {
      selectedRoom = snapshot.rooms.at(-1)?.id ?? '';
      roomName = ''; roomUrl = ''; showRoomForm = false;
    }
  }
  async function record(event: SubmitEvent) {
    event.preventDefault();
    await mutate('/api/recordings', { room_id: selectedRoom || snapshot.rooms[0]?.id, ends_at: Math.floor(new Date(endTime).getTime() / 1000) });
  }
  onMount(() => {
    let stopped = false;
    let timeout: ReturnType<typeof setTimeout>;
    async function poll() {
      await refresh();
      if (!stopped) timeout = setTimeout(poll, 2000);
    }
    void poll();
    return () => { stopped = true; clearTimeout(timeout); };
  });
</script>

<svelte:head><title>Room Replay · CESNET DVR</title></svelte:head>
<header class="flex h-20 items-center justify-between border-b border-line px-5 sm:px-7 xl:px-[max(28px,calc((100vw-1230px)/2))]">
  <a class="flex items-center gap-3 text-lg font-bold tracking-tight no-underline sm:text-xl" href="/" aria-label="Room Replay home"><span class="grid size-9 place-items-center rounded-xl bg-brand text-3xl text-white">↺</span> Room Replay</a>
  <span class="flex items-center gap-2 text-[10px] text-muted sm:text-xs"><span class={["size-2 rounded-full", connected ? "bg-emerald-500" : "bg-amber-500"]}></span>{connected ? 'Server connected' : 'Connecting to server'}</span>
</header>
<main class="mx-auto max-w-[1286px] px-4 py-8 sm:px-7 sm:py-10">
  <div class="mb-7 flex items-center justify-between gap-5"><div><p class="mb-2 text-[10px] font-bold tracking-[0.16em] text-brand-soft">YOUR CESNET RECORDINGS</p><h1 class="mb-3 text-4xl font-semibold tracking-[-0.04em] sm:text-[43px]">On your time.</h1><p class="text-sm text-muted">Capture a room. Start at the beginning. Catch up when you’re ready.</p></div><span class="hidden shrink-0 rounded-full border border-violet-200 px-3 py-1.5 text-[11px] text-brand-soft sm:block">First playback build</span></div>
  <aside class="mb-6 rounded-lg border border-stone-200 bg-stone-100 px-4 py-3 text-xs text-stone-600">Manual recording and playback are ready to try. Weekly scheduling, automatic storage cleanup, and saved viewing progress are still being built.</aside>
  {#if error}<div role="alert" class="mb-5 flex items-center justify-between rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">{error}<button aria-label="Dismiss error" onclick={() => error = ''}>×</button></div>{/if}
  <div class="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
    <section class="flex min-w-0 flex-col gap-6">
      {#if selected?.playable}
        <section class="overflow-hidden rounded-xl border border-line bg-white">
          <div class="flex items-center justify-between gap-3 border-b border-line px-5 py-5 sm:px-6"><div><p class="mb-2 text-[10px] font-bold tracking-[0.16em] text-brand-soft">NOW WATCHING</p><h2>{selected.title}</h2></div><button onclick={() => selectedRecording = ''}>Close player</button></div>
          {#key selected.id}<Player recording={selected} />{/key}
        </section>
      {/if}
      <section class="overflow-hidden rounded-xl border border-line bg-white">
        <div class="flex items-center justify-between gap-3 border-b border-line px-5 py-5 sm:px-6"><h2>Recordings <span class="ml-1 rounded bg-violet-50 px-2 py-0.5 align-middle text-xs text-brand-soft">{playable.length}</span></h2><span class="my-2 text-xs text-muted">Newest first</span></div>
        {#if playable.length === 0}
          <div class="px-5 py-14 text-center sm:py-20 [&_p]:mt-3 [&_p]:text-sm [&_p]:leading-7 [&_p]:text-muted"><span class="mx-auto mb-6 grid size-16 place-items-center rounded-2xl bg-violet-50 text-4xl text-brand-soft">▷</span><h3>Your next lecture belongs here.</h3><p>Add a room and start a recording.<br />You can watch as soon as the first segments arrive.</p></div>
        {:else}
          <div class="divide-y divide-line">
            {#each playable as recording (recording.id)}
              <article class={["flex items-center gap-3 border-b border-line p-4 last:border-0 sm:gap-4 sm:p-6", selectedRecording === recording.id && "bg-violet-50"]}>
                <div class="grid h-11 w-9 shrink-0 place-items-center rounded-lg bg-violet-50 text-xl text-brand-soft sm:size-12">{isActive(recording) ? '●' : '▷'}</div>
                <div class="min-w-0 flex-1"><h3>{recording.title}</h3><p class="mt-1 text-[11px] text-muted">{clock(recording.started_at)} · {minutes(recording.duration)}</p><div class="mt-2 flex gap-2"><span class={["rounded px-2 py-0.5 text-[10px]", isActive(recording) ? "bg-emerald-50 text-emerald-700" : "bg-stone-100 text-muted"]}>{label(recording.phase)}</span>{#if recording.incomplete}<span class="rounded bg-amber-50 px-2 py-0.5 text-[10px] text-amber-700">Incomplete</span>{/if}</div>{#if recording.message}<p class="mt-2 text-xs text-muted">{recording.message}</p>{/if}</div>
                <button class="shrink-0 text-brand" disabled={!recording.playable} onclick={() => selectedRecording = recording.id}>{recording.playable ? 'Watch' : 'Waiting…'}</button>
              </article>
            {/each}
          </div>
        {/if}
      </section>
      {#if history.length}
        <section class="overflow-hidden rounded-xl border border-line bg-white"><div class="flex items-center justify-between gap-3 border-b border-line px-5 py-5 sm:px-6"><h2>Recording history</h2></div>{#each history as recording}<div class="flex flex-wrap gap-3 px-6 py-4 text-xs text-muted"><strong>{recording.title}</strong><span>{recording.message ?? label(recording.phase)}</span><small>{clock(recording.started_at)}</small></div>{/each}</section>
      {/if}
    </section>
    <aside class="order-first flex flex-col gap-6 sm:grid sm:grid-cols-2 sm:items-start lg:order-last lg:flex lg:items-stretch">
      <section class="rounded-xl border border-line bg-white p-6">
        <p class="mb-2 text-[10px] font-bold tracking-[0.16em] text-brand-soft">{active ? 'CAPTURE IN PROGRESS' : 'MAKE TIME FOR LATER'}</p><h2>{active ? active.title : 'Record a room'}</h2>
        {#if active}
          <p>Ends {clock(active.ends_at)}</p><p class="my-2 text-xs text-muted">Recording continues when you close this page.</p>
          <button class="my-3 w-full border-red-200 bg-red-50 text-red-700 hover:bg-red-100" disabled={busy} onclick={() => active && mutate(`/api/recordings/${active.id}/stop`, {})}>Stop recording</button>
          <form onsubmit={event => { event.preventDefault(); if (active) void mutate(`/api/recordings/${active.id}/extend`, { ends_at: Math.floor(new Date(extensionTime).getTime() / 1000) }); }}>
            <label>New end time<input type="datetime-local" bind:value={extensionTime} required /></label><button class="w-full" disabled={busy}>Extend recording</button>
          </form>
        {:else}
          <form onsubmit={record}>
            <label>Room<select bind:value={selectedRoom} required><option value="" disabled>Choose a saved room</option>{#each snapshot.rooms as room}<option value={room.id}>{room.name}</option>{/each}</select></label>
            <label>Stop recording at<input type="datetime-local" bind:value={endTime} required /></label>
            <p class="-mt-1 text-[11px] text-muted">Times use this device’s timezone. Maximum 24 hours.</p>
            <button class="w-full border-brand bg-brand text-white hover:bg-brand-dark" disabled={busy || !snapshot.rooms.length}>● &nbsp; Record now</button>
          </form>
        {/if}
      </section>
      <section class="overflow-hidden rounded-xl border border-line bg-white px-5 pb-5 [&>.flex:first-child]:border-0 [&>.flex:first-child]:px-0">
        <div class="flex items-center justify-between gap-3 border-b border-line px-5 py-5 sm:px-6"><h2>Saved rooms</h2><button class="px-2 py-1 text-xs" onclick={() => showRoomForm = !showRoomForm}>{showRoomForm ? 'Cancel' : '+ Add'}</button></div>
        {#each snapshot.rooms as room}<div class="flex items-center gap-3 py-2.5 [&_strong]:block [&_strong]:text-xs [&_a]:text-[10px] [&_a]:text-muted"><span class="rounded-lg bg-violet-50 px-2.5 py-1 text-xl text-brand-soft">▦</span><div><strong>{room.name}</strong><a href={room.page_url} target="_blank" rel="noreferrer">CESNET source ↗</a></div></div>{/each}
        {#if !snapshot.rooms.length && !showRoomForm}<p class="my-2 text-xs text-muted">Save the rooms you want to record.</p><button class="w-full" onclick={() => showRoomForm = true}>Add your first room</button>{/if}
        {#if showRoomForm}<form onsubmit={saveRoom}><label>Room name<input bind:value={roomName} placeholder="MUNI FI A318" maxlength="100" required /></label><label>CESNET page or playlist URL<input type="url" bind:value={roomUrl} placeholder="https://live.cesnet.cz/munifia318.html" required /></label><button class="w-full border-brand bg-brand text-white hover:bg-brand-dark" disabled={busy}>{busy ? 'Saving…' : 'Save room'}</button></form>{/if}
      </section>
      <p class="hidden text-center text-[11px] leading-7 text-muted lg:block">One room at a time.<br />Your recordings stay on your server.</p>
    </aside>
  </div>
</main>
<footer class="mx-auto flex max-w-[1230px] justify-between gap-4 border-t border-line px-4 py-6 text-[10px] tracking-wide text-muted">ROOM REPLAY <span>Personal CESNET DVR · Development build</span></footer>
