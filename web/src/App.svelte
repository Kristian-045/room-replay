<script lang="ts">
  import { onMount } from 'svelte';
  import { request, isActive, type Snapshot } from './api';
  import Player from './player/Player.svelte';
  import { readSaved, save } from './player/progress';

  import { applyTheme, savedTheme, type Theme } from './theme';

  let theme = $state<Theme>(savedTheme());
  $effect(() => applyTheme(theme));
  let snapshot = $state<Snapshot>({ rooms: [], subjects: [], recordings: [] });
  let error = $state('');
  let busy = $state(false);
  let connected = $state(false);
  let roomName = $state('');
  let roomUrl = $state('');
  let selectedRoom = $state('');
  let selectedRecording = $state(readSaved('selected-recording') ?? '');
  $effect(() => save('selected-recording', selectedRecording));
  let subjectFilter = $state('');
  let filterDialog = $state<HTMLDialogElement>();
  let filterOpen = $state(false);
  const subjectOptions = $derived([{ id: '', name: 'All subjects' }, ...snapshot.subjects, { id: 'unassigned', name: 'Unassigned' }]);
  const filterLabel = $derived(subjectFilter === 'unassigned' ? 'Unassigned' : subjectFilter || 'Subjects');
  $effect(() => {
    if (!filterOpen) return;
    const overflow = document.documentElement.style.overflow;
    document.documentElement.style.overflow = 'hidden';
    return () => { document.documentElement.style.overflow = overflow; };
  });
  let selectedSubject = $state('');
  let showRoomForm = $state(false);
  let endTime = $state(localInput(Date.now() + 60 * 60 * 1000));
  let extensionTime = $state('');
  const active = $derived(snapshot.recordings.find(isActive));
  const filtered = $derived(snapshot.recordings.filter(recording => !subjectFilter || (subjectFilter === 'unassigned' ? !recording.subject_id : recording.subject_id === subjectFilter)));
  const playable = $derived(filtered.filter(recording => recording.playable || isActive(recording)));
  const history = $derived(filtered.filter(recording => !recording.playable && !isActive(recording)));
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
    await mutate('/api/recordings', { room_id: selectedRoom, subject_id: selectedSubject || null, ends_at: Math.floor(new Date(endTime).getTime() / 1000) });
  }
  function chooseSubject() {
    const subject = snapshot.subjects.find(subject => subject.id === selectedSubject);
    if (subject) selectedRoom = snapshot.rooms.find(room => room.page_url === subject.weekly_slot.room_page_url)?.id ?? '';
  }
  async function removeRoom(id: string) {
    if (await mutate(`/api/rooms/${id}/remove`, {})) {
      if (selectedRoom === id) selectedRoom = '';
    }
  }
  function openFilters() {
    filterDialog?.showModal();
    filterOpen = true;
  }
  function selectFilter(id: string, close: boolean) {
    subjectFilter = id;
    if (close) filterDialog?.close();
  }
  function dismissBackdrop(event: MouseEvent) {
    if (event.target !== filterDialog || !filterDialog) return;
    const bounds = filterDialog.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) filterDialog.close();
  }
  onMount(() => {
    const preference = matchMedia('(prefers-color-scheme: dark)');
    const updateTheme = () => applyTheme(theme);
    preference.addEventListener('change', updateTheme);
    const desktop = matchMedia('(min-width: 1024px)');
    const closeFiltersOnDesktop = () => { if (desktop.matches) filterDialog?.close(); };
    desktop.addEventListener('change', closeFiltersOnDesktop);
    let stopped = false;
    let timeout: ReturnType<typeof setTimeout>;
    async function poll() {
      await refresh();
      if (!stopped) timeout = setTimeout(poll, 2000);
    }
    void poll();
    return () => { stopped = true; clearTimeout(timeout); preference.removeEventListener('change', updateTheme); desktop.removeEventListener('change', closeFiltersOnDesktop); };
  });
</script>

<svelte:head><title>Recordings · Room Replay</title></svelte:head>
{#snippet subjectChoices(closeOnSelect = false)}
  <div role="group" aria-label="Filter by subject" class="flex flex-col gap-1">
    {#each subjectOptions as subject (subject.id)}
      <button
        class={["min-h-11 w-full px-3 py-2 text-left", subjectFilter === subject.id ? 'border-selection-line bg-selected text-brand hover:bg-selection-hover' : 'border-transparent hover:bg-surface-hover']}
        aria-pressed={subjectFilter === subject.id}
        onclick={() => selectFilter(subject.id, closeOnSelect)}
      >
        {#if subject.id && subject.id !== 'unassigned'}<span class="block font-semibold">{subject.id}</span>{/if}
        <span class="block text-xs">{subject.name}</span>
      </button>
    {/each}
  </div>
{/snippet}
<dialog id="subject-filter-dialog" bind:this={filterDialog} aria-labelledby="mobile-filter-heading" class="subject-sheet" onclose={() => filterOpen = false} onclick={dismissBackdrop}>
  <div class="mx-auto mt-3 h-1 w-10 shrink-0 rounded-full bg-line" aria-hidden="true"></div>
  <div class="flex shrink-0 items-center justify-between gap-3 border-b border-line px-5 py-4">
    <h2 id="mobile-filter-heading">Choose a subject</h2>
    <button class="min-h-11 min-w-11" aria-label="Close subject filter" onclick={() => filterDialog?.close()}>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
    </button>
  </div>
  <div class="min-h-0 overflow-y-auto overscroll-contain px-4 pt-3 pb-[max(16px,env(safe-area-inset-bottom))]">{@render subjectChoices(true)}</div>
</dialog>
<header class="flex h-16 items-center justify-between border-b border-line px-5 sm:px-7 xl:px-[max(28px,calc((100vw-1230px)/2))]">
  <a class="flex items-center gap-2 text-base font-semibold tracking-tight no-underline sm:text-lg" href="/"><img src="/favicon.svg?v=2" width="28" height="28" alt="" class="shrink-0" /><span>Room Replay</span></a>
  <div class="flex items-center gap-3">
    <select aria-label="Color theme" class="w-auto py-1.5 text-xs" bind:value={theme}>
      <option value="system">System theme</option><option value="light">Light mode</option><option value="dark">Dark mode</option>
    </select>
  <span class="flex items-center gap-2 text-xs text-muted" aria-label={connected ? 'Connected' : 'Connecting…'}><span class={["size-2 rounded-full", connected ? "bg-emerald-500" : "bg-amber-500"]}></span><span class="hidden sm:inline">{connected ? 'Connected' : 'Connecting…'}</span></span></div>
</header>
<main class="mx-auto max-w-[1286px] px-4 py-6 sm:px-7 sm:py-8">
  {#if error}
    <div role="alert" class="mb-5 flex items-center justify-between gap-3 rounded-lg border border-danger-line bg-danger-soft px-4 py-3 text-sm text-danger">{error}<button aria-label="Dismiss error" onclick={() => error = ''}>×</button></div>
  {/if}
  <div class="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
    <section class="flex min-w-0 flex-col gap-5">
      {#if selected?.playable}
        <section class="overflow-hidden rounded-xl border border-line bg-surface">
          <div class="flex items-center justify-between gap-3 px-5 py-4"><h2>{selected.title}</h2><button class="shrink-0" onclick={() => selectedRecording = ''}>Close player</button></div>
          {#key selected.id}<Player recording={selected} />{/key}
        </section>
      {/if}
      <section aria-label="Recordings" class="overflow-hidden rounded-xl border border-line bg-surface">
        <div class="flex items-center justify-between gap-3 border-b border-line px-4 py-3 text-xs text-muted sm:px-5">
          <span>{playable.length} {playable.length === 1 ? 'recording' : 'recordings'}<span class="hidden sm:inline lg:hidden"> · Newest first</span></span>
          <span class="hidden lg:inline">Newest first</span>
          <button class={["flex min-h-11 max-w-[60%] items-center gap-2 lg:hidden", subjectFilter && 'border-selection-line bg-selected text-brand']} aria-label={`Filter by subject: ${subjectFilter || 'All subjects'}`} aria-haspopup="dialog" aria-controls="subject-filter-dialog" aria-expanded={filterOpen} onclick={openFilters}>
            <svg class="shrink-0" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 5h16l-6 7v6l-4 2v-8z" /></svg>
            <span class="truncate">{filterLabel}</span>
          </button>
        </div>
        {#if playable.length === 0}
          <p class="px-5 py-12 text-center text-sm text-muted">{subjectFilter ? 'No recordings for this subject.' : 'No recordings yet.'}</p>
        {:else}
          <div class="divide-y divide-line">
            {#each playable as recording (recording.id)}
              <article data-recording-id={recording.id} class={["p-4 sm:p-5", selectedRecording === recording.id && 'bg-selected']}>
                <div class="flex items-start justify-between gap-4">
                  <div class="min-w-0"><h3>{recording.title}</h3><p class="mt-1 text-xs text-muted">{recording.room_name || recording.title} · {clock(recording.started_at)} · {minutes(recording.duration)}</p></div>
                  <div class="flex shrink-0 flex-col items-end gap-2 sm:flex-row sm:items-center">
                    {#if recording.playable}
                      <a class="rounded-lg border border-line px-3 py-2 text-xs text-brand hover:bg-surface-hover" href={`/api/recordings/${recording.id}/download`} download title={isActive(recording) ? 'Save the footage captured so far as MP4. Recording continues.' : 'Save this recording as MP4'}>{isActive(recording) ? 'Download so far' : 'Download MP4'}</a>
                    {/if}
                    <button class="text-brand" disabled={!recording.playable} onclick={() => selectedRecording = recording.id}>{recording.playable ? 'Watch' : 'Waiting…'}</button>
                  </div>
                </div>
                <div class="mt-3 flex flex-wrap items-center justify-between gap-3">
                  <div class="flex gap-2"><span class={["rounded px-2 py-0.5 text-[10px]", isActive(recording) ? 'bg-success-soft text-success' : 'bg-surface-hover text-muted']}>{label(recording.phase)}</span>{#if recording.incomplete}<span class="rounded bg-warning-soft px-2 py-0.5 text-[10px] text-warning">Incomplete</span>{/if}</div>
                  <select class="w-full py-1.5 text-xs sm:w-64" aria-label={`Subject for ${recording.title}`} value={recording.subject_id ?? ''} disabled={busy} onchange={event => mutate(`/api/recordings/${recording.id}/subject`, { subject_id: event.currentTarget.value || null })}>
                    <option value="">Unassigned</option>{#each snapshot.subjects as subject}<option value={subject.id}>{subject.id} · {subject.name}</option>{/each}
                  </select>
                </div>
                {#if recording.message}<p class="mt-2 text-xs text-muted">{recording.message}</p>{/if}
                {#if recording.playable && isActive(recording)}<p class="mt-2 text-xs text-muted">Download so far saves the available footage. Recording continues.</p>{/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>
      {#if history.length}
        <section class="overflow-hidden rounded-xl border border-line bg-surface"><h2 class="border-b border-line px-5 py-4">Recording history</h2>
          {#each history as recording}<div class="flex flex-wrap gap-3 px-5 py-4 text-xs text-muted"><strong>{recording.title}</strong><span>{recording.message ?? label(recording.phase)}</span><small>{clock(recording.started_at)}</small></div>{/each}
        </section>
      {/if}
    </section>
    <aside class="flex min-w-0 flex-col gap-5">
      <section class="hidden rounded-xl border border-line bg-surface p-5 lg:block">
        <h2>Filter by subject</h2>
        <div class="mt-3">{@render subjectChoices()}</div>
      </section>
      <details class="rounded-xl border border-line bg-surface px-5 py-4">
        <summary class="cursor-pointer text-sm font-semibold">{active ? 'Recording' : 'Record now'}</summary>
        {#if active}
          <p class="mt-3 text-sm font-medium">{active.title}</p><p class="mt-1 text-xs text-muted">Ends {clock(active.ends_at)}</p>
          <button class="my-3 w-full border-danger-line bg-danger-soft text-danger hover:bg-danger-hover" disabled={busy} onclick={() => active && mutate(`/api/recordings/${active.id}/stop`, {})}>Stop recording</button>
          <form onsubmit={event => { event.preventDefault(); if (active) void mutate(`/api/recordings/${active.id}/extend`, { ends_at: Math.floor(new Date(extensionTime).getTime() / 1000) }); }}>
            <label>New end time<input type="datetime-local" bind:value={extensionTime} required /></label><button class="w-full" disabled={busy}>Extend recording</button>
          </form>
        {:else}
          <form onsubmit={record}>
            <label>Subject<select aria-label="Subject" bind:value={selectedSubject} onchange={chooseSubject}><option value="">Unassigned</option>{#each snapshot.subjects as subject}<option value={subject.id}>{subject.id} · {subject.name}</option>{/each}</select></label>
            <label>Room<select aria-label="Room" bind:value={selectedRoom} required><option value="" disabled>Choose a room</option>{#each snapshot.rooms as room}<option value={room.id}>{room.name}</option>{/each}</select></label>
            <label>Stop recording at<input type="datetime-local" bind:value={endTime} required /></label>
            <button class="w-full border-brand bg-action text-white hover:bg-brand-dark" disabled={busy || !selectedRoom}>Record now</button>
          </form>
        {/if}
      </details>
      <details class="rounded-xl border border-line bg-surface px-5 py-4">
        <summary class="cursor-pointer text-sm font-semibold">Rooms</summary>
        <div class="flex justify-end py-3"><button class="px-2 py-1 text-xs" onclick={() => showRoomForm = !showRoomForm}>{showRoomForm ? 'Cancel' : '+ Add'}</button></div>
        {#each snapshot.rooms as room}
          <div class="flex items-center justify-between gap-3 border-t border-line py-3">
            <a class="text-sm hover:text-brand" href={room.page_url} target="_blank" rel="noreferrer">{room.name}</a>
            <button class="border-transparent px-2 py-1 text-xs text-muted hover:bg-danger-soft hover:text-danger" aria-label={`Remove ${room.name}`} disabled={busy || active?.room_id === room.id} title={active?.room_id === room.id ? 'Stop recording before removing this room' : 'Remove room; existing recordings are kept'} onclick={() => removeRoom(room.id)}>Remove</button>
          </div>
        {/each}
        {#if !snapshot.rooms.length && !showRoomForm}<p class="mb-3 text-xs text-muted">No saved rooms.</p><button class="w-full" onclick={() => showRoomForm = true}>Add room</button>{/if}
        {#if showRoomForm}<form onsubmit={saveRoom}><label>Room name<input bind:value={roomName} placeholder="FI A318" maxlength="100" required /></label><label>CESNET page or playlist URL<input type="url" bind:value={roomUrl} placeholder="https://live.cesnet.cz/munifia318.html" required /></label><button class="w-full border-brand bg-action text-white hover:bg-brand-dark" disabled={busy}>{busy ? 'Saving…' : 'Save room'}</button></form>{/if}
      </details>
      <details class="rounded-xl border border-line bg-surface px-5 py-4">
        <summary class="cursor-pointer text-sm font-semibold">Subjects</summary>
        {#each snapshot.subjects as subject}
          <div class="mt-4 border-t border-line pt-3"><p class="text-xs font-semibold">{subject.id} · {subject.name}</p><p class="mt-1 text-[11px] text-muted">{subject.weekly_slot.day} {subject.weekly_slot.starts_at}–{subject.weekly_slot.ends_at} · {subject.weekly_slot.room}</p>{#if !subject.weekly_slot.room_page_url}<p class="mt-1 text-[11px] text-warning">Stream link not configured</p>{/if}</div>
        {/each}
      </details>
    </aside>
  </div>
</main>
