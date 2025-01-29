add-migration:
	sea migrate -d src/sea-migrations/ generate $(NAME)

run-migrations:
	sea migrate -d src/sea-migrations/
	sea generate entity --output-dir src/db-entities/src/ --lib

snoop-ble:
	adb shell "nc -s 127.0.0.1 -p 8872 -L system/bin/tail -f -c +0 data/misc/bluetooth/logs/btsnoop_hci.log"