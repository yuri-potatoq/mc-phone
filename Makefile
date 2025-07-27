export DATABASE_URL="sqlite://mc-phone.db?mode=rwc"

run/migration:
	sqlx migrate run
