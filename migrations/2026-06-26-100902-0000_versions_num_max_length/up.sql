ALTER TABLE versions
    ADD CONSTRAINT versions_num_max_length
    CHECK (char_length(num) <= 150) NOT VALID;
