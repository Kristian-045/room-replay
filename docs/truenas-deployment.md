# TrueNAS SCALE 25.10 deployment

Room Replay is installed as the TrueNAS custom app `room-replay` on pool `pool`. Open http://192.0.2.10:3000 from a device on the tailnet. On 2026-09-29, TrueNAS reported one running container, and Brave and Chromium loaded the UI over Tailscale. The scheduler update to image 0.2.0 was installed later that day; the existing PV017 recording resumed under the new container.

The application data lives in `tank/room-replay` at `/mnt/tank/room-replay`. It is owned by UID/GID 568 and has a 100 GiB ZFS dataset quota. The custom app uses [truenas-compose.yaml](../deploy/truenas-compose.yaml), binds port 3000 only to the host's Tailscale address `192.0.2.10`, and runs as UID/GID 568 with a read-only root filesystem. No application password is configured. The app did not answer on the NAS LAN address during validation.

The deployed image is `room-replay:0.2.0`, built from the repository [Dockerfile](../Dockerfile). The admin loaded its image archive, then the app was created through the TrueNAS `app.create` API. The archive remains at `room-replay.tar.gz`; it can be removed after confirming the app remains healthy, since Docker now has the image.

The deployment started with an empty library by choice. Today's PV017 session was then started on the NAS for the remaining class time before the scheduler update. The laptop's existing recording and configuration were not migrated. Browser-local playback progress stays on each browser and does not migrate.

To update, build a newly tagged image on the laptop, transfer its archive to TrueNAS, load it into Docker, then change the image tag in the custom app YAML through the TrueNAS UI or `app.update` API. Keep the dataset mounted at `/data`. Only run one app instance against that dataset.

The four FI timetable entries run automatically in Europe/Bratislava time, including after the browser closes. FAST entries have no source links and do not run. Automatic cleanup is not implemented. The ZFS quota limits the dataset to 100 GiB, but a full dataset can interrupt a recording. Monitor storage use until app-level cleanup exists.
