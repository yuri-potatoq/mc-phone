export DATABASE_URL="sqlite://mc-phone.db"

run/migration:
	sqlx migrate run
