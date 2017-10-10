DROP TRIGGER trigger_emails_reconfirm ON emails;
DROP TRIGGER trigger_emails_set_token_generated_at ON emails;
DROP FUNCTION reconfirm_email_on_email_change();
DROP FUNCTION emails_set_token_generated_at();

CREATE TABLE tokens (
  id SERIAL PRIMARY KEY,
  email_id INTEGER NOT NULL UNIQUE REFERENCES emails (id),
  token TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

INSERT INTO tokens (email_id, token, created_at)
SELECT id, token, token_generated_at FROM emails WHERE token_generated_at IS NOT NULL;

ALTER TABLE emails DROP COLUMN token;
ALTER TABLE emails DROP COLUMN token_generated_at;
