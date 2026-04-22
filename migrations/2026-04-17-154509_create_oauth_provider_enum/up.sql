-- safety-assured:start
CREATE TYPE oauth_provider AS ENUM ('github');
-- safety-assured:end

comment on type oauth_provider is 'OAuth identity providers supported by crates.io';
