-- Introduce a new column `expiry_notification_at` in the `api_tokens` table.
-- This column will hold the timestamp of when the user was informed about their token's impending expiration.
ALTER TABLE api_tokens ADD expiry_notification_at TIMESTAMP;

