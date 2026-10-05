# TrueNAS SCALE deployment

This project can run as a TrueNAS custom app using the example in `deploy/truenas-compose.yaml`. Edit the image tag, host data directory, and bind address for your NAS before installing it.

Build the image on a machine with Docker:

```sh
docker build -t room-replay:0.5.1 .
docker save room-replay:0.5.1 | gzip > room-replay.tar.gz
```

Transfer the archive to the NAS, load it with `docker load`, and create a persistent dataset for `/data`. Make that directory writable by UID/GID 568, then install the compose file as a custom app. Bind port 3000 to loopback or to a private tailnet address. Do not expose this app directly to the public internet because it has no login.

To update, build and transfer a newly tagged image, load it on the NAS, and update the image tag in the custom app. Keep the `/data` mount in place so the library and recordings persist. Run one app instance per data directory.

The dataset quota limits total storage, but the app does not yet remove old recordings automatically. Monitor available space because a full dataset can interrupt a recording.
