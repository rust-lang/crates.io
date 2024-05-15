-- Remove the `expiry_notification_at` column from the `api_tokens` table.
ALTER TABLE api_tokens DROP expiry_notification_at;
