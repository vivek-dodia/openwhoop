# OpenWhoop

OpenWhoop is project that allows you to download heart rate data directly from your Whoop4.0 device without Whoop subscription or Whoops servers, making data your own.

### How to Run?

First create empty sqlite db and copy `.env.example`
```sh
sqlite3 db.sqlite "VACUUM;"
cp .env.example .env
```

After that scan until you find your Whoop device:
```sh
cargo run -r -- scan
```

After you find your device copy its address to `.env` under `WHOOP_ADDR`, and you can download data from your whoop by running:
```sh
cargo run -r -- download-history
```


## TODO:

- [ ] Sleep detection, for most of things like strain, recovery, HRV, etc..., I have been able to reverse engineer calculations, but I need reverse engineer sleep detection and activity detection before they can be automatically calculated
- [ ] Mobile/Desktop app
- [ ] Sp02 readings
- [ ] Temperature readings