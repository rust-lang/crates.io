ALTER TABLE api_tokens ADD expiry_notification_at TIMESTAMP;

COMMENT ON COLUMN api_tokens.expiry_notification_at IS 'timestamp of when the user was informed about their token''s impending expiration';
