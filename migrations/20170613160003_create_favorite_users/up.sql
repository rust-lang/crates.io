CREATE TABLE favorite_users (
            user_id               INTEGER NOT NULL,
            target_id             INTEGER NOT NULL,
            CONSTRAINT favorites_pkey PRIMARY KEY (user_id, target_id),
            CONSTRAINT fk_favorites_user_id FOREIGN KEY (user_id)
                REFERENCES users (id),
            CONSTRAINT fk_favorites_target_id FOREIGN KEY (target_id)
                REFERENCES users (id)
          );