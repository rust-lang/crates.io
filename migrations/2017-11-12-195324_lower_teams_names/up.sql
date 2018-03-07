UPDATE teams SET login=lower(login);
ALTER TABLE teams
    ADD CONSTRAINT teams_login_lowercase_ck
    CHECK (login = lower(login));
